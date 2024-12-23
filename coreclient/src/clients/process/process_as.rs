// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Result;
use openmls::prelude::MlsMessageOut;
use phnxtypes::{
    crypto::hpke::HpkeDecryptable,
    identifiers::QualifiedGroupId,
    messages::{
        client_as::{EncryptedConnectionEstablishmentPackage, ExtractedAsQueueMessagePayload},
        client_ds::{InfraAadMessage, InfraAadPayload, JoinConnectionGroupParamsAad},
        client_ds_out::ExternalCommitInfoIn,
        QueueMessage,
    },
};
use tls_codec::DeserializeBytes;

use crate::{
    clients::connection_establishment::{
        ConnectionEstablishmentPackageIn, ConnectionEstablishmentPackageTbs,
    },
    groups::Group,
};

use super::{
    anyhow, AsCredentials, Asset, Contact, Conversation, ConversationAttributes, ConversationId,
    CoreUser, EarEncryptable, FriendshipPackage, InfraCredentialSigningKey, SignatureEarKey,
    UserProfile,
};
use crate::key_stores::queue_ratchets::StorableAsQueueRatchet;

impl CoreUser {
    /// Decrypt a `QueueMessage` received from the AS queue.
    pub async fn decrypt_as_queue_message(
        &self,
        as_message_ciphertext: QueueMessage,
    ) -> Result<ExtractedAsQueueMessagePayload> {
        let mut connection = self.inner.connection.lock().await;
        let transaction = connection.transaction()?;
        let mut as_queue_ratchet = StorableAsQueueRatchet::load(&transaction)?;

        let payload = as_queue_ratchet.decrypt(as_message_ciphertext)?;

        as_queue_ratchet.update_ratchet(&transaction)?;
        transaction.commit()?;

        Ok(payload.extract()?)
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

                // Create signature ear key
                let signature_ear_key = SignatureEarKey::random()?;

                // Prepare group
                let (leaf_signer, aad, qgid) =
                    self.prepare_group(&signature_ear_key, &cep_tbs, own_user_profile)?;

                // Fetch external commit info
                let eci = self.fetch_external_commit_info(&cep_tbs, &qgid).await?;

                // Join group
                let (group, commit, group_info) = self
                    .join_group_externally(signature_ear_key, eci, &cep_tbs, leaf_signer, aad)
                    .await?;

                // Create conversation
                let (mut conversation, contact) =
                    self.create_connection_conversation(&group, &cep_tbs)?;

                // Store group, conversation & contact
                self.store_group_conversation_contact(&group, &mut conversation, contact, &cep_tbs)
                    .await?;

                // Send confirmation
                self.send_confirmation_to_ds(&group, commit, group_info, &cep_tbs, qgid)
                    .await?;

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
            self.inner.connection.clone(),
            &self.inner.api_clients,
            &sender_domain,
            cep_in.sender_credential().signer_fingerprint(),
        )
        .await?;
        cep_in
            .verify(as_intermediate_credential.verifying_key())
            .map_err(|e| {
                log::error!("Error verifying connection establishment package: {}", e);
                anyhow!("Error verifying connection establishment package")
            })
    }

    fn prepare_group(
        &self,
        signature_ear_key: &SignatureEarKey,
        cep_tbs: &ConnectionEstablishmentPackageTbs,
        own_user_profile: UserProfile,
    ) -> Result<(InfraCredentialSigningKey, InfraAadMessage, QualifiedGroupId)> {
        // We create a new group and signal that fact to the user,
        // so the user can decide if they want to accept the
        // connection.

        let leaf_signer = InfraCredentialSigningKey::generate(
            &self.inner.key_store.signing_key,
            &signature_ear_key,
        );
        let esek =
            signature_ear_key.encrypt(&cep_tbs.connection_group_signature_ear_key_wrapper_key)?;

        let encrypted_friendship_package = FriendshipPackage {
            friendship_token: self.inner.key_store.friendship_token.clone(),
            add_package_ear_key: self.inner.key_store.add_package_ear_key.clone(),
            client_credential_ear_key: self.inner.key_store.client_credential_ear_key.clone(),
            signature_ear_key_wrapper_key: self
                .inner
                .key_store
                .signature_ear_key_wrapper_key
                .clone(),
            wai_ear_key: self.inner.key_store.wai_ear_key.clone(),
            user_profile: own_user_profile,
        }
        .encrypt(&cep_tbs.friendship_package_ear_key)?;
        let ecc = self
            .inner
            .key_store
            .signing_key
            .credential()
            .encrypt(&cep_tbs.connection_group_credential_key)?;

        let aad: InfraAadMessage =
            InfraAadPayload::JoinConnectionGroup(JoinConnectionGroupParamsAad {
                encrypted_client_information: (ecc, esek),
                encrypted_friendship_package,
            })
            .into();
        let qgid =
            QualifiedGroupId::tls_deserialize_exact_bytes(cep_tbs.connection_group_id.as_slice())?;

        Ok((leaf_signer, aad, qgid))
    }

    async fn load_own_user_profile(&self) -> Result<UserProfile> {
        let connection = self.inner.connection.lock().await;
        Ok(UserProfile::load(&connection, &self.user_name())
            // We unwrap here, because we know that the user exists.
            .map(|user_option| user_option.unwrap())?)
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
        signature_ear_key: SignatureEarKey,
        eci: ExternalCommitInfoIn,
        cep_tbs: &ConnectionEstablishmentPackageTbs,
        leaf_signer: InfraCredentialSigningKey,
        aad: InfraAadMessage,
    ) -> Result<(Group, MlsMessageOut, MlsMessageOut)> {
        let (group, commit, group_info) = Group::join_group_externally(
            self.inner.connection.clone(),
            &self.inner.api_clients,
            eci,
            leaf_signer,
            signature_ear_key,
            cep_tbs.connection_group_ear_key.clone(),
            cep_tbs
                .connection_group_signature_ear_key_wrapper_key
                .clone(),
            cep_tbs.connection_group_credential_key.clone(),
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
        group: &Group,
        conversation: &mut Conversation,
        contact: Contact,
        cep_tbs: &ConnectionEstablishmentPackageTbs,
    ) -> Result<()> {
        let connection = self.inner.connection.lock().await;
        group.store(&connection)?;
        conversation.store(&connection)?;
        // Store the user profile of the sender.
        cep_tbs.friendship_package.user_profile.store(&connection)?;
        // TODO: For now, we automatically confirm conversations.
        conversation.confirm(&connection)?;
        // TODO: Here, we want to store a contact
        contact.store(&connection)?;
        Ok(())
    }

    async fn send_confirmation_to_ds(
        &self,
        group: &Group,
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
                group.user_auth_key().ok_or(anyhow!("No user auth key"))?,
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
