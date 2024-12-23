// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Result;
use phnxtypes::{
    crypto::hpke::HpkeDecryptable,
    identifiers::QualifiedGroupId,
    messages::{
        client_as::ExtractedAsQueueMessagePayload,
        client_ds::{InfraAadPayload, JoinConnectionGroupParamsAad},
        client_ds_out::ExternalCommitInfoIn,
        QueueMessage,
    },
};
use tls_codec::DeserializeBytes;

use crate::{clients::connection_establishment::ConnectionEstablishmentPackageIn, groups::Group};

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
        let conversation_id = match as_message_plaintext {
            ExtractedAsQueueMessagePayload::EncryptedConnectionEstablishmentPackage(ecep) => {
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
                let cep_tbs = cep_in.verify(as_intermediate_credential.verifying_key());
                // We create a new group and signal that fact to the user,
                // so the user can decide if they want to accept the
                // connection.

                let signature_ear_key = SignatureEarKey::random()?;
                let leaf_signer = InfraCredentialSigningKey::generate(
                    &self.inner.key_store.signing_key,
                    &signature_ear_key,
                );
                let esek = signature_ear_key
                    .encrypt(&cep_tbs.connection_group_signature_ear_key_wrapper_key)?;

                // EncryptedConnectionEstablishmentPackage Phase 2: Load the user profile
                let connection = self.inner.connection.lock().await;
                let own_user_profile = UserProfile::load(&connection, &self.user_name())
                    // We unwrap here, because we know that the user exists.
                    .map(|user_option| user_option.unwrap())?;
                drop(connection);

                let encrypted_friendship_package = FriendshipPackage {
                    friendship_token: self.inner.key_store.friendship_token.clone(),
                    add_package_ear_key: self.inner.key_store.add_package_ear_key.clone(),
                    client_credential_ear_key: self
                        .inner
                        .key_store
                        .client_credential_ear_key
                        .clone(),
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

                let aad = InfraAadPayload::JoinConnectionGroup(JoinConnectionGroupParamsAad {
                    encrypted_client_information: (ecc, esek),
                    encrypted_friendship_package,
                })
                .into();

                // EncryptedConnectionEstablishmentPackage Phase 3: Fetch external commit information.
                let qgid = QualifiedGroupId::tls_deserialize_exact_bytes(
                    cep_tbs.connection_group_id.as_slice(),
                )?;
                let eci: ExternalCommitInfoIn = self
                    .inner
                    .api_clients
                    .get(qgid.owning_domain())?
                    .ds_connection_group_info(
                        cep_tbs.connection_group_id.clone(),
                        &cep_tbs.connection_group_ear_key,
                    )
                    .await?;

                // EncryptedConnectionEstablishmentPackage Phase 4: Join the group.
                let (group, commit, group_info) = Group::join_group_externally(
                    self.inner.connection.clone(),
                    &self.inner.api_clients,
                    eci,
                    leaf_signer,
                    signature_ear_key,
                    cep_tbs.connection_group_ear_key.clone(),
                    cep_tbs.connection_group_signature_ear_key_wrapper_key,
                    cep_tbs.connection_group_credential_key,
                    aad,
                    self.inner.key_store.signing_key.credential(),
                )
                .await?;

                // EncryptedConnectionEstablishmentPackage Phase 5: Store the group and the conversation.
                let connection = self.inner.connection.lock().await;
                group.store(&connection)?;
                let sender_client_id = cep_tbs.sender_client_credential.identity();
                let conversation_picture_option = cep_tbs
                    .friendship_package
                    .user_profile
                    .profile_picture()
                    .map(|asset| match asset {
                        Asset::Value(value) => value.to_owned(),
                    });
                let mut conversation = Conversation::new_connection_conversation(
                    group.group_id().clone(),
                    sender_client_id.user_name().clone(),
                    ConversationAttributes::new(
                        sender_client_id.user_name().to_string(),
                        conversation_picture_option,
                    ),
                )?;
                conversation.store(&connection)?;
                // Store the user profile of the sender.
                cep_tbs.friendship_package.user_profile.store(&connection)?;
                // TODO: For now, we automatically confirm conversations.
                conversation.confirm(&connection)?;
                // TODO: Here, we want to store a contact
                Contact::from_friendship_package(
                    sender_client_id,
                    conversation.id(),
                    cep_tbs.friendship_package,
                )
                .store(&connection)?;
                drop(connection);

                let qs_client_reference = self.create_own_client_reference();

                // EncryptedConnectionEstablishmentPackage Phase 6: Send the
                // confirmation by way of commit and group info to the DS.
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
                conversation.id()
            }
        };
        Ok(conversation_id)
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
