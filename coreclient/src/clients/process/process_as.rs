// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::{Context, Result, ensure};
use openmls::prelude::MlsMessageOut;
use phnxcommon::{
    credentials::keys::ClientSigningKey,
    crypto::{hpke::HpkeDecryptable, indexed_aead::keys::UserProfileKey},
    identifiers::{QualifiedGroupId, UserHandle},
    messages::{
        client_as::ConnectionOfferMessage,
        client_ds::{InfraAadMessage, InfraAadPayload, JoinConnectionGroupParamsAad},
        client_ds_out::ExternalCommitInfoIn,
    },
};
use phnxprotos::auth_service::v1::{HandleQueueMessage, handle_queue_message};
use sqlx::SqliteConnection;
use sqlx::SqliteTransaction;
use tls_codec::DeserializeBytes;
use tracing::error;

use crate::{
    clients::connection_offer::{ConnectionOfferIn, payload::ConnectionOfferPayload},
    groups::{Group, ProfileInfo},
    key_stores::indexed_keys::StorableIndexedKey,
    store::StoreNotifier,
    utils::connection_ext::ConnectionExt,
};

use super::{
    AsCredentials, Contact, Conversation, ConversationAttributes, ConversationId, CoreUser,
    EarEncryptable, FriendshipPackage, anyhow,
};

impl CoreUser {
    /// Process a queue message received from the AS handle queue.
    ///
    /// Returns the [`ConversationId`] of any newly created conversations.
    pub async fn process_handle_queue_message(
        &self,
        user_handle: &UserHandle,
        handle_queue_message: HandleQueueMessage,
    ) -> Result<ConversationId> {
        let payload = handle_queue_message
            .payload
            .context("no payload in handle queue message")?;
        match payload {
            handle_queue_message::Payload::ConnectionOffer(eco) => {
                self.process_connection_offer(user_handle.clone(), eco.try_into()?)
                    .await
            }
        }
    }

    async fn process_connection_offer(
        &self,
        handle: UserHandle,
        ecep: ConnectionOfferMessage,
    ) -> Result<ConversationId> {
        let mut connection = self.pool().acquire().await?;

        // Parse & verify connection offer
        let cep_payload = self
            .parse_and_verify_connection_offer(&mut connection, ecep, handle.clone())
            .await?;

        // Prepare group
        let own_user_profile_key = UserProfileKey::load_own(&mut *connection).await?;
        let (aad, qgid) = self.prepare_group(&cep_payload, &own_user_profile_key)?;

        // Fetch external commit info
        let eci = self.fetch_external_commit_info(&cep_payload, &qgid).await?;

        // Join group
        let (group, commit, group_info, mut member_profile_info) = self
            .join_group_externally(&mut connection, eci, &cep_payload, self.signing_key(), aad)
            .await?;

        // Verify that the group has only one other member and that it's
        // the sender of the CEP.
        let members = group.members(&mut *connection).await;

        ensure!(
            members.len() == 2,
            "Connection group has more than two members: {:?}",
            members
        );

        ensure!(
            members.contains(self.user_id())
                && members.contains(cep_payload.sender_client_credential.identity()),
            "Connection group has unexpected members: {:?}",
            members
        );

        // There should be only one user profile
        let contact_profile_info = member_profile_info
            .pop()
            .context("No user profile returned when joining connection group")?;

        debug_assert!(
            member_profile_info.is_empty(),
            "More than one user profile returned when joining connection group"
        );

        // Fetch and store user profile

        self.with_notifier(async |notifier| {
            self.fetch_and_store_user_profile(&mut connection, notifier, contact_profile_info)
                .await
        })
        .await?;

        // Create conversation
        // Note: For now, the conversation is immediately confirmed.
        let (mut conversation, contact) = self
            .create_connection_conversation(&mut connection, &group, &cep_payload)
            .await?;

        let mut notifier = self.store_notifier();

        // Store group, conversation & contact
        connection
            .with_transaction(async |txn| {
                self.store_group_conversation_contact(
                    txn,
                    &mut notifier,
                    &group,
                    &mut conversation,
                    contact,
                )
                .await
            })
            .await?;

        // Send confirmation
        self.send_confirmation_to_ds(commit, group_info, &cep_payload, qgid)
            .await?;

        notifier.notify();

        // Return the conversation ID
        Ok(conversation.id())
    }

    /// Parse and verify the connection offer
    async fn parse_and_verify_connection_offer(
        &self,
        connection: &mut SqliteConnection,
        com: ConnectionOfferMessage,
        user_handle: UserHandle,
    ) -> Result<ConnectionOfferPayload> {
        // TODO: Fetch the right key based on the hash in the ConnectionOfferMessage.
        let (ecep, hash) = com.into_parts();
        let cep_in = ConnectionOfferIn::decrypt(
            ecep,
            &self.inner.key_store.connection_decryption_key,
            &[],
            &[],
        )?;
        // Fetch authentication AS credentials of the sender if we don't have them already.
        let sender_domain = cep_in.sender_domain();

        // EncryptedConnectionOffer Phase 1: Load the AS credential of the sender.
        let as_intermediate_credential = AsCredentials::get(
            connection,
            &self.inner.api_clients,
            sender_domain,
            cep_in.signer_fingerprint(),
        )
        .await?;
        cep_in
            .verify(
                as_intermediate_credential.verifying_key(),
                user_handle,
                hash,
            )
            .map_err(|error| {
                error!(%error, "Error verifying connection offer");
                anyhow!("Error verifying connection offer")
            })
    }

    fn prepare_group(
        &self,
        cep_payload: &ConnectionOfferPayload,
        own_user_profile_key: &UserProfileKey,
    ) -> Result<(InfraAadMessage, QualifiedGroupId)> {
        // We create a new group and signal that fact to the user,
        // so the user can decide if they want to accept the
        // connection.

        let encrypted_user_profile_key = own_user_profile_key.encrypt(
            &cep_payload.connection_group_identity_link_wrapper_key,
            self.user_id(),
        )?;

        let encrypted_friendship_package = FriendshipPackage {
            friendship_token: self.inner.key_store.friendship_token.clone(),
            connection_key: self.inner.key_store.connection_key.clone(),
            wai_ear_key: self.inner.key_store.wai_ear_key.clone(),
            user_profile_base_secret: own_user_profile_key.base_secret().clone(),
        }
        .encrypt(&cep_payload.friendship_package_ear_key)?;

        let aad: InfraAadMessage =
            InfraAadPayload::JoinConnectionGroup(JoinConnectionGroupParamsAad {
                encrypted_friendship_package,
                encrypted_user_profile_key,
            })
            .into();
        let qgid = QualifiedGroupId::tls_deserialize_exact_bytes(
            cep_payload.connection_group_id.as_slice(),
        )?;

        Ok((aad, qgid))
    }

    async fn fetch_external_commit_info(
        &self,
        cep_payload: &ConnectionOfferPayload,
        qgid: &QualifiedGroupId,
    ) -> Result<ExternalCommitInfoIn> {
        Ok(self
            .inner
            .api_clients
            .get(qgid.owning_domain())?
            .ds_connection_group_info(
                cep_payload.connection_group_id.clone(),
                &cep_payload.connection_group_ear_key, //
            )
            .await?)
    }

    async fn join_group_externally(
        &self,
        connection: &mut SqliteConnection,
        eci: ExternalCommitInfoIn,
        cep_payload: &ConnectionOfferPayload,
        leaf_signer: &ClientSigningKey,
        aad: InfraAadMessage,
    ) -> Result<(Group, MlsMessageOut, MlsMessageOut, Vec<ProfileInfo>)> {
        let (group, commit, group_info, member_profile_info) = Group::join_group_externally(
            &mut *connection,
            &self.inner.api_clients,
            eci,
            leaf_signer,
            cep_payload.connection_group_ear_key.clone(),
            cep_payload
                .connection_group_identity_link_wrapper_key
                .clone(),
            aad,
            self.inner.key_store.signing_key.credential(),
        )
        .await?;
        Ok((group, commit, group_info, member_profile_info))
    }

    async fn create_connection_conversation(
        &self,
        connection: &mut SqliteConnection,
        group: &Group,
        cep_payload: &ConnectionOfferPayload,
    ) -> Result<(Conversation, Contact)> {
        let sender_user_id = cep_payload.sender_client_credential.identity();

        let display_name = self
            .user_profile_internal(connection, sender_user_id)
            .await
            .display_name;

        let conversation = Conversation::new_connection_conversation(
            group.group_id().clone(),
            sender_user_id.clone(),
            ConversationAttributes::new(display_name.to_string(), None),
        )?;
        let contact = Contact::from_friendship_package(
            sender_user_id.clone(),
            conversation.id(),
            cep_payload.friendship_package.clone(),
        )?;
        Ok((conversation, contact))
    }

    async fn store_group_conversation_contact(
        &self,
        txn: &mut SqliteTransaction<'_>,
        notifier: &mut StoreNotifier,
        group: &Group,
        conversation: &mut Conversation,
        contact: Contact,
    ) -> Result<()> {
        group.store(txn.as_mut()).await?;
        conversation.store(txn.as_mut(), notifier).await?;
        contact.upsert(txn.as_mut(), notifier).await?;
        Ok(())
    }

    async fn send_confirmation_to_ds(
        &self,
        commit: MlsMessageOut,
        group_info: MlsMessageOut,
        cep_payload: &ConnectionOfferPayload,
        qgid: QualifiedGroupId,
    ) -> Result<()> {
        let qs_client_reference = self.create_own_client_reference();
        self.inner
            .api_clients
            .get(qgid.owning_domain())?
            .ds_join_connection_group(
                commit,
                group_info,
                qs_client_reference,
                &cep_payload.connection_group_ear_key,
            )
            .await?;
        Ok(())
    }
}
