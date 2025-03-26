// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::anyhow;
use phnxtypes::{
    codec::PhnxCodec,
    crypto::{
        ear::keys::FriendshipPackageEarKey, hpke::HpkeEncryptable, signatures::signable::Signable,
    },
    identifiers::QualifiedUserName,
    messages::client_as::UserConnectionPackagesParams,
};
use tracing::info;

use crate::{
    Conversation, ConversationAttributes, ConversationId, PartialContact, UserProfile,
    clients::connection_establishment::{ConnectionEstablishmentPackageTbs, FriendshipPackage},
    groups::{Group, openmls_provider::PhnxOpenMlsProvider},
    key_stores::as_credentials::AsCredentials,
};

use super::CoreUser;

impl CoreUser {
    /// Create a connection with a new user.
    ///
    /// Returns the [`ConversationId`] of the newly created connection
    /// conversation.
    pub(crate) async fn add_contact(
        &self,
        user_name: QualifiedUserName,
    ) -> anyhow::Result<ConversationId> {
        let params = UserConnectionPackagesParams {
            user_name: user_name.clone(),
        };
        // Phase 1: Fetch connection key packages from the AS
        let user_domain = user_name.domain();
        info!(%user_name, "Adding contact");
        let user_key_packages = self
            .inner
            .api_clients
            .get(&user_domain)?
            .as_user_connection_packages(params)
            .await?;

        // The AS should return an error if the user does not exist, but we
        // check here locally just to be sure.
        if user_key_packages.connection_packages.is_empty() {
            return Err(anyhow!("User {user_name} does not exist"));
        }
        // Phase 2: Verify the connection key packages
        info!("Verifying connection packages");
        let mut verified_connection_packages = vec![];
        for connection_package in user_key_packages.connection_packages.into_iter() {
            let as_intermediate_credential = AsCredentials::get(
                self.pool(),
                &self.inner.api_clients,
                &user_domain,
                connection_package.client_credential_signer_fingerprint(),
            )
            .await?;
            let verifying_key = as_intermediate_credential.verifying_key();
            verified_connection_packages.push(connection_package.verify(verifying_key)?)
        }

        // TODO: Connection Package Validation
        // * Version
        // * Lifetime

        // Phase 3: Request a group id from the DS
        info!("Requesting group id");
        let group_id = self
            .inner
            .api_clients
            .default_client()?
            .ds_request_group_id()
            .await?;

        // Phase 4: Prepare the connection locally
        info!("Creating local connection group");
        let title = format!("Connection group: {} - {}", self.user_name(), user_name);
        let conversation_attributes = ConversationAttributes::new(title.to_string(), None);
        let group_data = PhnxCodec::to_vec(&conversation_attributes)?.into();
        let (connection_group, partial_params) = self
            .with_transaction(async |transaction| {
                let provider = PhnxOpenMlsProvider::new(transaction);
                let (group, group_membership, partial_params) = Group::create_group(
                    &provider,
                    &self.inner.key_store.signing_key,
                    &self.inner.key_store.connection_key,
                    group_id.clone(),
                    group_data,
                )?;
                group_membership.store(&mut *transaction).await?;
                group.store(&mut *transaction).await?;
                Ok((group, partial_params))
            })
            .await?;

        // TODO: Once we allow multi-client, invite all our other clients to the
        // connection group.

        let own_user_profile = UserProfile::load(self.pool(), &self.user_name())
            .await
            // We unwrap here, because we know that the user exists.
            .map(|user_option| user_option.unwrap())?;

        // Create the connection conversation
        let conversation = Conversation::new_connection_conversation(
            group_id.clone(),
            user_name.clone(),
            conversation_attributes,
        )?;
        let mut notifier = self.store_notifier();
        conversation.store(self.pool(), &mut notifier).await?;

        let friendship_package = FriendshipPackage {
            friendship_token: self.inner.key_store.friendship_token.clone(),
            key_package_ear_key: self.inner.key_store.key_package_ear_key.clone(),
            connection_key: self.inner.key_store.connection_key.clone(),
            wai_ear_key: self.inner.key_store.wai_ear_key.clone(),
            user_profile: own_user_profile,
        };

        let friendship_package_ear_key = FriendshipPackageEarKey::random()?;

        // Create and persist a new partial contact
        PartialContact::new(
            user_name.clone(),
            conversation.id(),
            friendship_package_ear_key.clone(),
        )
        .store(self.pool(), &mut notifier)
        .await?;

        // Store the user profile of the partial contact (we don't have a
        // display name or a profile picture yet)
        UserProfile::new(user_name, None, None)
            .store(self.pool(), &mut notifier)
            .await?;

        // Create a connection establishment package
        let connection_establishment_package = ConnectionEstablishmentPackageTbs {
            sender_client_credential: self.inner.key_store.signing_key.credential().clone(),
            connection_group_id: group_id,
            connection_group_ear_key: connection_group.group_state_ear_key().clone(),
            connection_group_identity_link_wrapper_key: connection_group
                .identity_link_wrapper_key()
                .clone(),
            friendship_package_ear_key,
            friendship_package,
        }
        .sign(&self.inner.key_store.signing_key)?;

        let client_reference = self.create_own_client_reference();
        let params = partial_params.into_params(client_reference);

        // Phase 5: Create the connection group on the DS and send off the
        // connection establishment packages
        info!("Creating connection group on DS");
        self.inner
            .api_clients
            .default_client()?
            .ds_create_group(
                params,
                connection_group.leaf_signer(),
                connection_group.group_state_ear_key(),
            )
            .await?;

        // Encrypt the connection establishment package for each connection and send it off.
        for connection_package in verified_connection_packages {
            let ciphertext = connection_establishment_package.encrypt(
                connection_package.encryption_key(),
                &[],
                &[],
            );
            let client_id = connection_package.client_credential().identity();

            self.inner
                .api_clients
                .get(&user_domain)?
                .as_enqueue_message(client_id, ciphertext)
                .await?;
        }

        notifier.notify();

        Ok(conversation.id())
    }
}
