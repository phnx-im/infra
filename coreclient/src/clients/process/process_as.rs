// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Result;
use openmls::prelude::MlsMessageOut;
use phnxtypes::{
    crypto::hpke::HpkeDecryptable,
    identifiers::QualifiedGroupId,
    messages::{
        QueueMessage,
        client_as::{EncryptedConnectionEstablishmentPackage, ExtractedAsQueueMessagePayload},
        client_ds::{InfraAadMessage, InfraAadPayload, JoinConnectionGroupParamsAad},
        client_ds_out::ExternalCommitInfoIn,
    },
};
use tls_codec::DeserializeBytes;
use tracing::error;

use crate::{
    clients::connection_establishment::{
        ConnectionEstablishmentPackageIn, ConnectionEstablishmentPackageTbs,
    },
    groups::Group,
    key_stores::leaf_keys::LeafKeys,
    store::StoreNotifier,
};

use super::{
    AsCredentials, Asset, Contact, Conversation, ConversationAttributes, ConversationId, CoreUser,
    EarEncryptable, FriendshipPackage, UserProfile, anyhow,
};
use crate::key_stores::queue_ratchets::StorableAsQueueRatchet;

impl CoreUser {
    /// Decrypt a `QueueMessage` received from the AS queue.
    pub async fn decrypt_as_queue_message(
        &self,
        as_message_ciphertext: QueueMessage,
    ) -> Result<ExtractedAsQueueMessagePayload> {
        self.with_transaction(async |transaction| {
            let mut as_queue_ratchet = StorableAsQueueRatchet::load(&mut **transaction).await?;
            let payload = as_queue_ratchet.decrypt(as_message_ciphertext)?;
            as_queue_ratchet.update_ratchet(&mut **transaction).await?;
            Ok(payload.extract()?)
        })
        .await
    }

    /// Process a decrypted message received from the AS queue.
    ///
    /// Returns the [`ConversationId`] of any newly created conversations.
    pub async fn process_as_message(
        &self,
        as_message_plaintext: ExtractedAsQueueMessagePayload,
    ) -> Result<ConversationId> {
        match as_message_plaintext {
            ExtractedAsQueueMessagePayload::EncryptedConnectionEstablishmentPackage(ecep) => {
                // Parse & verify connection establishment package
                let cep_tbs = self
                    .parse_and_verify_connection_establishment_package(ecep)
                    .await?;

                // Load user profile
                let own_user_profile = self.load_own_user_profile().await?;

                // Prepare group
                let (leaf_keys, aad, qgid) = self.prepare_group(&cep_tbs, own_user_profile)?;

                // Fetch external commit info
                let eci = self.fetch_external_commit_info(&cep_tbs, &qgid).await?;

                // Join group
                let (group, commit, group_info) = self
                    .join_group_externally(eci, &cep_tbs, leaf_keys, aad)
                    .await?;

                // Create conversation
                let (mut conversation, contact) =
                    self.create_connection_conversation(&group, &cep_tbs)?;

                let mut notifier = self.store_notifier();

                // Store group, conversation & contact
                self.store_group_conversation_contact(
                    &mut notifier,
                    &group,
                    &mut conversation,
                    contact,
                    &cep_tbs,
                )
                .await?;

                // Send confirmation
                self.send_confirmation_to_ds(commit, group_info, &cep_tbs, qgid)
                    .await?;

                notifier.notify();

                // Return the conversation ID
                Ok(conversation.id())
            }
        }
    }

    /// Parse and verify the connection establishment package.
    async fn parse_and_verify_connection_establishment_package(
        &self,
        ecep: EncryptedConnectionEstablishmentPackage,
    ) -> Result<ConnectionEstablishmentPackageTbs> {
        let cep_in = ConnectionEstablishmentPackageIn::decrypt(
            ecep,
            &self.inner.key_store.connection_decryption_key,
            &[],
            &[],
        )?;
        // Fetch authentication AS credentials of the sender if we
        // don't have them already.
        let sender_domain = cep_in.sender_credential().domain();

        // EncryptedConnectionEstablishmentPackage Phase 1: Load the
        // AS credential of the sender.
        let as_intermediate_credential = AsCredentials::get(
            self.pool(),
            &self.inner.api_clients,
            &sender_domain,
            cep_in.sender_credential().signer_fingerprint(),
        )
        .await?;
        cep_in
            .verify(as_intermediate_credential.verifying_key())
            .map_err(|error| {
                error!(%error, "Error verifying connection establishment package");
                anyhow!("Error verifying connection establishment package")
            })
    }

    fn prepare_group(
        &self,
        cep_tbs: &ConnectionEstablishmentPackageTbs,
        own_user_profile: UserProfile,
    ) -> Result<(LeafKeys, InfraAadMessage, QualifiedGroupId)> {
        // We create a new group and signal that fact to the user,
        // so the user can decide if they want to accept the
        // connection.

        let leaf_keys = LeafKeys::generate(
            &self.inner.key_store.signing_key,
            &self.inner.key_store.connection_key,
        )?;

        let encrypted_identity_link_key = leaf_keys
            .identity_link_key()
            .encrypt(&cep_tbs.connection_group_identity_link_wrapper_key)?;

        let encrypted_friendship_package = FriendshipPackage {
            friendship_token: self.inner.key_store.friendship_token.clone(),
            key_package_ear_key: self.inner.key_store.key_package_ear_key.clone(),
            connection_key: self.inner.key_store.connection_key.clone(),
            wai_ear_key: self.inner.key_store.wai_ear_key.clone(),
            user_profile: own_user_profile,
        }
        .encrypt(&cep_tbs.friendship_package_ear_key)?;

        let aad: InfraAadMessage =
            InfraAadPayload::JoinConnectionGroup(JoinConnectionGroupParamsAad {
                encrypted_friendship_package,
                encrypted_identity_link_key,
            })
            .into();
        let qgid =
            QualifiedGroupId::tls_deserialize_exact_bytes(cep_tbs.connection_group_id.as_slice())?;

        Ok((leaf_keys, aad, qgid))
    }

    async fn load_own_user_profile(&self) -> Result<UserProfile> {
        // We unwrap here, because we know that the user exists.
        Ok(UserProfile::load(self.pool(), &self.user_name())
            .await?
            .unwrap())
    }

    async fn fetch_external_commit_info(
        &self,
        cep_tbs: &ConnectionEstablishmentPackageTbs,
        qgid: &QualifiedGroupId,
    ) -> Result<ExternalCommitInfoIn> {
        Ok(self
            .inner
            .api_clients
            .get(qgid.owning_domain())?
            .ds_connection_group_info(
                cep_tbs.connection_group_id.clone(),
                &cep_tbs.connection_group_ear_key,
            )
            .await?)
    }

    async fn join_group_externally(
        &self,
        eci: ExternalCommitInfoIn,
        cep_tbs: &ConnectionEstablishmentPackageTbs,
        leaf_keys: LeafKeys,
        aad: InfraAadMessage,
    ) -> Result<(Group, MlsMessageOut, MlsMessageOut)> {
        let (leaf_signer, identity_link_key) = leaf_keys.into_parts();
        let (group, commit, group_info) = Group::join_group_externally(
            self.pool(),
            &self.inner.api_clients,
            eci,
            leaf_signer,
            identity_link_key,
            cep_tbs.connection_group_ear_key.clone(),
            cep_tbs.connection_group_identity_link_wrapper_key.clone(),
            aad,
            self.inner.key_store.signing_key.credential(),
        )
        .await?;
        Ok((group, commit, group_info))
    }

    fn create_connection_conversation(
        &self,
        group: &Group,
        cep_tbs: &ConnectionEstablishmentPackageTbs,
    ) -> Result<(Conversation, Contact)> {
        let sender_client_id = cep_tbs.sender_client_credential.identity();
        let conversation_picture_option = cep_tbs
            .friendship_package
            .user_profile
            .profile_picture()
            .map(|asset| match asset {
                Asset::Value(value) => value.to_owned(),
            });
        let conversation = Conversation::new_connection_conversation(
            group.group_id().clone(),
            sender_client_id.user_name().clone(),
            ConversationAttributes::new(
                sender_client_id.user_name().to_string(),
                conversation_picture_option,
            ),
        )?;
        let contact = Contact::from_friendship_package(
            sender_client_id,
            conversation.id(),
            cep_tbs.friendship_package.clone(),
        );
        Ok((conversation, contact))
    }

    async fn store_group_conversation_contact(
        &self,
        notifier: &mut StoreNotifier,
        group: &Group,
        conversation: &mut Conversation,
        contact: Contact,
        cep_tbs: &ConnectionEstablishmentPackageTbs,
    ) -> Result<()> {
        let mut connection = self.pool().acquire().await?;
        group.store(&mut *connection).await?;
        conversation.store(&mut *connection, notifier).await?;
        // Store the user profile of the sender.
        cep_tbs
            .friendship_package
            .user_profile
            .store_or_ignore(&mut *connection, notifier)
            .await?;
        // TODO: For now, we automatically confirm conversations.
        conversation.confirm(&mut *connection, notifier).await?;
        // TODO: Here, we want to store a contact
        contact.store(&mut *connection, notifier).await?;
        Ok(())
    }

    async fn send_confirmation_to_ds(
        &self,
        commit: MlsMessageOut,
        group_info: MlsMessageOut,
        cep_tbs: &ConnectionEstablishmentPackageTbs,
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
                &cep_tbs.connection_group_ear_key,
            )
            .await?;
        Ok(())
    }

    /// Convenience function that takes a list of `QueueMessage`s retrieved from
    /// the AS, decrypts them, and processes them.
    pub async fn fully_process_as_messages(
        &self,
        as_messages: Vec<QueueMessage>,
    ) -> Result<Vec<ConversationId>> {
        let mut conversation_ids = vec![];
        for as_message in as_messages {
            let as_message_plaintext = self.decrypt_as_queue_message(as_message).await?;
            let conversation_id = self.process_as_message(as_message_plaintext).await?;
            conversation_ids.push(conversation_id);
        }
        Ok(conversation_ids)
    }
}
