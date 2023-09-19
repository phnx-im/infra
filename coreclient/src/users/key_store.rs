// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ops::Deref;

use anyhow::Result;
use phnxbackend::messages::{client_as::AsQueueMessagePayload, client_ds::QsQueueMessagePayload};

use crate::utils::persistence::{DataType, PersistenceError};

use super::*;

// For now we persist the key store along with the user. Any key material that gets rotated in the future needs to be persisted separately.
#[derive(Serialize, Deserialize)]
pub(crate) struct MemoryUserKeyStore {
    // Client credential secret key
    pub(super) signing_key: ClientSigningKey,
    // AS-specific key material
    pub(super) as_queue_decryption_key: RatchetDecryptionKey,
    pub(super) connection_decryption_key: ConnectionDecryptionKey,
    // QS-specific key material
    pub(super) qs_client_signing_key: QsClientSigningKey,
    pub(super) qs_user_signing_key: QsUserSigningKey,
    pub(super) qs_queue_decryption_key: RatchetDecryptionKey,
    pub(super) qs_client_id_encryption_key: ClientIdEncryptionKey,
    pub(super) push_token_ear_key: PushTokenEarKey,
    // These are keys that we send to our contacts
    pub(super) friendship_token: FriendshipToken,
    pub(super) add_package_ear_key: AddPackageEarKey,
    pub(super) client_credential_ear_key: ClientCredentialEarKey,
    pub(super) signature_ear_key_wrapper_key: SignatureEarKeyWrapperKey,
    pub(super) wai_ear_key: WelcomeAttributionInfoEarKey,
}

pub(crate) struct LeafKeyStore<'a> {
    db_connection: &'a Connection,
}

impl<'a> From<&'a Connection> for LeafKeyStore<'a> {
    fn from(db_connection: &'a Connection) -> Self {
        Self { db_connection }
    }
}

impl<'a> LeafKeyStore<'a> {
    pub(crate) fn get(
        &self,
        verifying_key: &SignaturePublicKey,
    ) -> Result<Option<PersistableLeafKeys>, PersistenceError> {
        let verifying_key_str = hex::encode(verifying_key.as_slice());
        PersistableLeafKeys::load_one(self.db_connection, Some(&verifying_key_str), None)
    }

    pub(crate) fn generate(&self, signing_key: &ClientSigningKey) -> Result<PersistableLeafKeys> {
        let signature_ear_key = SignatureEarKey::random()?;
        let leaf_signing_key = InfraCredentialSigningKey::generate(signing_key, &signature_ear_key);
        let keys = PersistableLeafKeys::from_connection_and_payload(
            self.db_connection,
            (leaf_signing_key, signature_ear_key),
        );
        keys.persist()?;
        Ok(keys)
    }

    pub(crate) fn delete(
        &self,
        verifying_key: &SignaturePublicKey,
    ) -> Result<(), PersistenceError> {
        let verifying_key_str = hex::encode(verifying_key.as_slice());
        PersistableLeafKeys::purge_key(self.db_connection, &verifying_key_str)
    }
}

pub(crate) struct PersistableLeafKeys<'a> {
    connection: &'a Connection,
    verifying_key_str: String,
    payload: (InfraCredentialSigningKey, SignatureEarKey),
}

impl PersistableLeafKeys<'_> {
    pub(crate) fn leaf_signing_key(&self) -> &InfraCredentialSigningKey {
        &self.payload.0
    }

    pub(crate) fn signature_ear_key(&self) -> &SignatureEarKey {
        &self.payload.1
    }
}

impl<'a> Persistable<'a> for PersistableLeafKeys<'a> {
    type Key = String;

    type SecondaryKey = String;

    const DATA_TYPE: DataType = DataType::LeafKeys;

    fn key(&self) -> &Self::Key {
        &self.verifying_key_str
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &self.verifying_key_str
    }

    type Payload = (InfraCredentialSigningKey, SignatureEarKey);

    fn connection(&self) -> &Connection {
        self.connection
    }

    fn payload(&self) -> &Self::Payload {
        &self.payload
    }

    fn from_connection_and_payload(conn: &'a Connection, payload: Self::Payload) -> Self {
        let verifying_key_str = hex::encode(payload.0.credential().verifying_key().as_slice());
        Self {
            connection: conn,
            verifying_key_str,
            payload,
        }
    }
}

pub(crate) struct QsVerifyingKeyStore<'a> {
    db_connection: &'a Connection,
    api_clients: ApiClients,
}

impl<'a> QsVerifyingKeyStore<'a> {
    pub(crate) fn new(db_connection: &'a Connection, api_clients: ApiClients) -> Self {
        Self {
            db_connection,
            api_clients,
        }
    }

    pub(crate) async fn get(&self, domain: &Fqdn) -> Result<PersistableQsVerifyingKey> {
        if let Some(verifying_key) =
            PersistableQsVerifyingKey::load_one(self.db_connection, Some(domain), None)?
        {
            Ok(verifying_key)
        } else {
            let verifying_key_response = self.api_clients.get(domain)?.qs_verifying_key().await?;
            let verifying_key = PersistableQsVerifyingKey::from_connection_and_payload(
                self.db_connection,
                QualifiedQsVerifyingKey {
                    qs_verifying_key: verifying_key_response.verifying_key,
                    domain: domain.clone(),
                },
            );
            verifying_key.persist()?;
            Ok(verifying_key)
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct QualifiedQsVerifyingKey {
    qs_verifying_key: QsVerifyingKey,
    domain: Fqdn,
}

pub(crate) struct PersistableQsVerifyingKey<'a> {
    connection: &'a Connection,
    payload: QualifiedQsVerifyingKey,
}

impl Deref for PersistableQsVerifyingKey<'_> {
    type Target = QsVerifyingKey;

    fn deref(&self) -> &Self::Target {
        &self.payload.qs_verifying_key
    }
}

impl<'a> Persistable<'a> for PersistableQsVerifyingKey<'a> {
    type Key = Fqdn;

    type SecondaryKey = Fqdn;

    const DATA_TYPE: DataType = DataType::QsVerifyingKey;

    fn key(&self) -> &Self::Key {
        &self.payload.domain
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &self.payload.domain
    }

    type Payload = QualifiedQsVerifyingKey;

    fn connection(&self) -> &Connection {
        self.connection
    }

    fn payload(&self) -> &Self::Payload {
        &self.payload
    }

    fn from_connection_and_payload(conn: &'a Connection, payload: Self::Payload) -> Self {
        Self {
            connection: conn,
            payload,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub(crate) enum QueueType {
    As,
    Qs,
}

impl std::fmt::Display for QueueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueueType::As => write!(f, "as"),
            QueueType::Qs => write!(f, "qs"),
        }
    }
}

pub(crate) struct QueueRatchetStore<'a> {
    db_connection: &'a Connection,
}

impl<'a> From<&'a Connection> for QueueRatchetStore<'a> {
    fn from(db_connection: &'a Connection) -> Self {
        Self { db_connection }
    }
}

impl<'a> QueueRatchetStore<'a> {
    fn initialize_sequence_number(&self, queue_type: QueueType) -> Result<(), PersistenceError> {
        log::info!("Initializing sequence number for queue type {}", queue_type);
        let sequence_number = PersistableSequenceNumber::from_connection_and_payload(
            self.db_connection,
            QualifiedSequenceNumber {
                queue_type,
                sequence_number: 0,
            },
        );
        sequence_number.persist()
    }

    pub(crate) fn initialize_as_queue_ratchet(&self, ratcht_secret: RatchetSecret) -> Result<()> {
        let payload = AsQueueRatchet::try_from(ratcht_secret)?;
        let ratchet =
            PersistableAsQueueRatchet::from_connection_and_payload(self.db_connection, payload);
        ratchet.persist()?;
        self.initialize_sequence_number(QueueType::As)?;
        Ok(())
    }

    pub(crate) fn initialize_qs_queue_ratchet(&self, ratcht_secret: RatchetSecret) -> Result<()> {
        let ratchet = QsQueueRatchet::try_from(ratcht_secret)?;
        let ratchet =
            PersistableQsQueueRatchet::from_connection_and_payload(self.db_connection, ratchet);
        ratchet.persist()?;
        self.initialize_sequence_number(QueueType::Qs)?;
        Ok(())
    }

    pub(crate) fn get_as_queue_ratchet(&self) -> Result<PersistableAsQueueRatchet> {
        PersistableAsQueueRatchet::load_one(self.db_connection, Some(&QueueType::As), None)?
            .ok_or(anyhow!("Couldn't find AS queue ratchet in DB."))
    }

    pub(crate) fn get_qs_queue_ratchet(&self) -> Result<PersistableQsQueueRatchet> {
        PersistableQsQueueRatchet::load_one(self.db_connection, Some(&QueueType::Qs), None)?
            .ok_or(anyhow!("Couldn't find QS queue ratchet in DB."))
    }

    pub(crate) fn get_sequence_number(
        &self,
        queue_type: QueueType,
    ) -> Result<PersistableSequenceNumber> {
        PersistableSequenceNumber::load_one(self.db_connection, Some(&queue_type), None)?
            .ok_or(anyhow!("No sequence number found for type {}", queue_type))
    }
}

pub(crate) struct PersistableAsQueueRatchet<'a> {
    connection: &'a Connection,
    payload: AsQueueRatchet,
}

impl PersistableAsQueueRatchet<'_> {
    pub(crate) fn decrypt(&mut self, ciphertext: QueueMessage) -> Result<AsQueueMessagePayload> {
        let plaintext = self.payload.decrypt(ciphertext)?;
        self.persist()?;
        Ok(plaintext)
    }
}

impl Deref for PersistableAsQueueRatchet<'_> {
    type Target = AsQueueRatchet;

    fn deref(&self) -> &Self::Target {
        &self.payload
    }
}

impl<'a> Persistable<'a> for PersistableAsQueueRatchet<'a> {
    type Key = QueueType;

    type SecondaryKey = QueueType;

    const DATA_TYPE: DataType = DataType::QueueRatchet;

    fn key(&self) -> &Self::Key {
        &QueueType::As
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &QueueType::As
    }

    type Payload = AsQueueRatchet;

    fn connection(&self) -> &Connection {
        self.connection
    }

    fn payload(&self) -> &Self::Payload {
        &self.payload
    }

    fn from_connection_and_payload(conn: &'a Connection, payload: Self::Payload) -> Self {
        Self {
            connection: conn,
            payload,
        }
    }
}

pub(crate) struct PersistableQsQueueRatchet<'a> {
    connection: &'a Connection,
    payload: QsQueueRatchet,
}

impl PersistableQsQueueRatchet<'_> {
    pub(crate) fn decrypt(&mut self, ciphertext: QueueMessage) -> Result<QsQueueMessagePayload> {
        let plaintext = self.payload.decrypt(ciphertext)?;
        self.persist()?;
        Ok(plaintext)
    }
}

impl<'a> Persistable<'a> for PersistableQsQueueRatchet<'a> {
    type Key = QueueType;

    type SecondaryKey = QueueType;

    const DATA_TYPE: DataType = DataType::QueueRatchet;

    fn key(&self) -> &Self::Key {
        &QueueType::Qs
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &QueueType::Qs
    }

    type Payload = QsQueueRatchet;

    fn connection(&self) -> &Connection {
        self.connection
    }

    fn payload(&self) -> &Self::Payload {
        &self.payload
    }

    fn from_connection_and_payload(conn: &'a Connection, payload: Self::Payload) -> Self {
        Self {
            connection: conn,
            payload,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct QualifiedSequenceNumber {
    queue_type: QueueType,
    sequence_number: u64,
}

pub(crate) struct PersistableSequenceNumber<'a> {
    connection: &'a Connection,
    payload: QualifiedSequenceNumber,
}

impl PersistableSequenceNumber<'_> {
    pub(crate) fn set(&mut self, sequence_number: u64) -> Result<()> {
        self.payload.sequence_number = sequence_number;
        self.persist()?;
        Ok(())
    }
}

impl Deref for PersistableSequenceNumber<'_> {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.payload.sequence_number
    }
}

impl<'a> Persistable<'a> for PersistableSequenceNumber<'a> {
    type Key = QueueType;

    type SecondaryKey = QueueType;

    const DATA_TYPE: DataType = DataType::SequenceNumber;

    fn key(&self) -> &Self::Key {
        &self.payload.queue_type
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &self.payload.queue_type
    }

    type Payload = QualifiedSequenceNumber;

    fn connection(&self) -> &Connection {
        self.connection
    }

    fn payload(&self) -> &Self::Payload {
        &self.payload
    }

    fn from_connection_and_payload(conn: &'a Connection, payload: Self::Payload) -> Self {
        Self {
            connection: conn,
            payload,
        }
    }
}
