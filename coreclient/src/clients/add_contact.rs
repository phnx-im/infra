// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Context;
use openmls::group::GroupId;
use phnxapiclient::{ApiClient, as_api::ConnectionOfferResponder};
use phnxcommon::{
    codec::PhnxCodec,
    credentials::keys::ClientSigningKey,
    crypto::{
        ear::keys::FriendshipPackageEarKey, hpke::HpkeEncryptable,
        indexed_aead::keys::UserProfileKey,
    },
    identifiers::{QsReference, UserHandle, UserId},
    messages::{client_as::ConnectionPackage, client_ds_out::CreateGroupParamsOut},
};
use sqlx::SqliteConnection;
use tracing::info;

use crate::{
    Conversation, ConversationAttributes, ConversationId,
    clients::connection_offer::FriendshipPackage,
    contacts::HandleContact,
    groups::{Group, PartialCreateGroupParams, openmls_provider::PhnxOpenMlsProvider},
    key_stores::{
        MemoryUserKeyStore, as_credentials::AsCredentials, indexed_keys::StorableIndexedKey,
    },
    store::StoreNotifier,
    utils::connection_ext::ConnectionExt as _,
};

use super::{
    CoreUser,
    connection_offer::{ConnectionOffer, payload::ConnectionOfferPayload},
};

impl CoreUser {
    /// Create a connection with via a user handle.
    pub(crate) async fn add_contact_via_handle(
        &self,
        handle: UserHandle,
    ) -> anyhow::Result<ConversationId> {
        let client = self.api_client()?;

        // Phase 1: Fetch a connection package from the AS
        let (connection_package, connection_offer_responder) =
            client.as_connect_handle(&handle).await?;

        // Phase 2: Verify the connection package
        let as_intermediate_credential = AsCredentials::get(
            self.pool().acquire().await?.as_mut(),
            &self.inner.api_clients,
            self.user_id().domain(),
            connection_package.client_credential_signer_fingerprint(),
        )
        .await?;
        let verifying_key = as_intermediate_credential.verifying_key();
        let connection_package = connection_package.verify(verifying_key)?;

        // Phase 3: Create a connection group
        let group_id = client.ds_request_group_id().await?;

        // Phase 4: Prepare the connection locally
        let mut connection = self.pool().acquire().await?;
        let mut notifier = self.store_notifier();
        let connection_package = VerifiedConnectionPackagesWithGroupId {
            verified_connection_packages: vec![connection_package],
            group_id,
        };

        let local_group = connection
            .with_transaction(async |txn| {
                connection_package
                    .create_local_connection_group(
                        txn,
                        &mut notifier,
                        &self.inner.key_store,
                        handle.clone(),
                    )
                    .await
            })
            .await?;

        let client_reference = self.create_own_client_reference();

        let local_partial_contact = local_group
            .create_handle_contact(
                &mut connection,
                &mut notifier,
                &self.inner.key_store,
                client_reference,
                self.user_id(),
                handle,
            )
            .await?;

        // Phase 5: Create the connection group on the DS and send off the connection offer
        let conversation_id = local_partial_contact
            .create_connection_group_via_handle(
                &client,
                self.signing_key(),
                connection_offer_responder,
            )
            .await?;

        notifier.notify();

        Ok(conversation_id)
    }
}

struct VerifiedConnectionPackagesWithGroupId {
    verified_connection_packages: Vec<ConnectionPackage>,
    group_id: GroupId,
}

impl VerifiedConnectionPackagesWithGroupId {
    async fn create_local_connection_group(
        self,
        txn: &mut sqlx::SqliteTransaction<'_>,
        notifier: &mut StoreNotifier,
        key_store: &MemoryUserKeyStore,
        handle: UserHandle,
    ) -> anyhow::Result<LocalGroup> {
        let Self {
            verified_connection_packages,
            group_id,
        } = self;

        info!("Creating local connection group");
        let title = format!("Connection group: {}", handle.plaintext());
        let conversation_attributes = ConversationAttributes::new(title, None);
        let group_data = PhnxCodec::to_vec(&conversation_attributes)?.into();

        let provider = PhnxOpenMlsProvider::new(txn);
        let (group, group_membership, partial_params) = Group::create_group(
            &provider,
            &key_store.signing_key,
            group_id.clone(),
            group_data,
        )?;
        group_membership.store(txn.as_mut()).await?;
        group.store(txn.as_mut()).await?;

        // TODO: Once we allow multi-client, invite all our other clients to the
        // connection group.

        // Create the connection conversation
        let conversation = Conversation::new_handle_conversation(
            group_id.clone(),
            conversation_attributes,
            handle.clone(),
        );
        conversation.store(txn.as_mut(), notifier).await?;

        Ok(LocalGroup {
            group,
            partial_params,
            conversation_id: conversation.id(),
            verified_connection_packages,
        })
    }
}

struct LocalGroup {
    group: Group,
    partial_params: PartialCreateGroupParams,
    conversation_id: ConversationId,
    verified_connection_packages: Vec<ConnectionPackage>,
}

impl LocalGroup {
    async fn create_handle_contact(
        self,
        connection: &mut SqliteConnection,
        notifier: &mut StoreNotifier,
        key_store: &MemoryUserKeyStore,
        own_client_reference: QsReference,
        own_user_id: &UserId,
        handle: UserHandle,
    ) -> anyhow::Result<LocalHandleContact> {
        let Self {
            group,
            partial_params,
            conversation_id,
            verified_connection_packages,
        } = self;

        let own_user_profile_key = UserProfileKey::load_own(&mut *connection).await?;

        let friendship_package = FriendshipPackage {
            friendship_token: key_store.friendship_token.clone(),
            connection_key: key_store.connection_key.clone(),
            wai_ear_key: key_store.wai_ear_key.clone(),
            user_profile_base_secret: own_user_profile_key.base_secret().clone(),
        };

        let friendship_package_ear_key = FriendshipPackageEarKey::random()?;

        // Create and persist a new partial handle contact
        HandleContact::new(
            handle.clone(),
            conversation_id,
            friendship_package_ear_key.clone(),
        )
        .upsert(&mut *connection, notifier)
        .await?;

        // Create a connection offer
        let connection_offer_payload = ConnectionOfferPayload {
            sender_client_credential: key_store.signing_key.credential().clone(),
            connection_group_id: group.group_id().clone(),
            connection_group_ear_key: group.group_state_ear_key().clone(),
            connection_group_identity_link_wrapper_key: group.identity_link_wrapper_key().clone(),
            friendship_package_ear_key,
            friendship_package,
        };
        let connection_offer = connection_offer_payload.sign(&key_store.signing_key, handle)?;

        let encrypted_user_profile_key =
            own_user_profile_key.encrypt(group.identity_link_wrapper_key(), own_user_id)?;
        let params = partial_params.into_params(own_client_reference, encrypted_user_profile_key);

        Ok(LocalHandleContact {
            group,
            connection_offer,
            params,
            conversation_id,
            verified_connection_packages,
        })
    }
}

struct LocalHandleContact {
    group: Group,
    connection_offer: ConnectionOffer,
    params: CreateGroupParamsOut,
    conversation_id: ConversationId,
    verified_connection_packages: Vec<ConnectionPackage>,
}

impl LocalHandleContact {
    async fn create_connection_group_via_handle(
        self,
        client: &ApiClient,
        signer: &ClientSigningKey,
        responder: ConnectionOfferResponder,
    ) -> anyhow::Result<ConversationId> {
        let Self {
            group,
            connection_offer,
            params,
            conversation_id,
            verified_connection_packages,
        } = self;

        info!("Creating connection group on DS");
        client
            .ds_create_group(params, signer, group.group_state_ear_key())
            .await?;

        // Encrypt the connection offer and send it off.
        debug_assert!(
            verified_connection_packages.len() == 1,
            "Only one connection package is supported for handle connections"
        );
        let connection_package = verified_connection_packages
            .into_iter()
            .next()
            .context("logic error: no connection package")?;
        let ciphertext = connection_offer.encrypt(connection_package.encryption_key(), &[], &[]);
        responder.send(ciphertext).await?;

        Ok(conversation_id)
    }
}
