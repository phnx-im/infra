// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use rusqlite::ToSql;

use super::{errors::RandomnessError, *};

#[cfg(test)]
mod tests;

pub trait RatchetPayload<Ciphertext: RatchetCiphertext>:
    EarEncryptable<RatchetKey, Ciphertext> + EarDecryptable<RatchetKey, Ciphertext>
{
}
pub trait RatchetCiphertext: AsRef<Ciphertext> + From<Ciphertext> {}

impl<Ciphertext: RatchetCiphertext, T> RatchetPayload<Ciphertext> for T where
    T: EarEncryptable<RatchetKey, Ciphertext> + EarDecryptable<RatchetKey, Ciphertext>
{
}
impl<T> RatchetCiphertext for T where T: AsRef<Ciphertext> + From<Ciphertext> {}

#[derive(
    Serialize, PartialEq, Deserialize, Clone, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize,
)]
pub struct QueueRatchet<Ciphertext: RatchetCiphertext, Payload: RatchetPayload<Ciphertext>> {
    sequence_number: u64,
    secret: RatchetSecret,
    key: RatchetKey,
    _phantom: PhantomData<(Ciphertext, Payload)>,
}

#[cfg(feature = "sqlite")]
impl<Ciphertext: RatchetCiphertext, Payload: RatchetPayload<Ciphertext>> ToSql
    for QueueRatchet<Ciphertext, Payload>
{
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::Owned(
            rusqlite::types::Value::Blob(
                serde_json::to_vec(self)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
            ),
        ))
    }
}

#[cfg(feature = "sqlite")]
impl<Ciphertext: RatchetCiphertext, Payload: RatchetPayload<Ciphertext>> rusqlite::types::FromSql
    for QueueRatchet<Ciphertext, Payload>
{
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let bytes = value.as_blob()?;
        serde_json::from_slice(bytes).map_err(|e| rusqlite::types::FromSqlError::Other(Box::new(e)))
    }
}

impl<Ciphertext: RatchetCiphertext, Payload: RatchetPayload<Ciphertext>> TryFrom<RatchetSecret>
    for QueueRatchet<Ciphertext, Payload>
{
    type Error = LibraryError;

    fn try_from(secret: RatchetSecret) -> Result<Self, Self::Error> {
        let key = RatchetKey::derive(&secret, Vec::new()).map_err(|_| LibraryError)?;
        Ok(Self {
            sequence_number: 0,
            secret,
            key,
            _phantom: PhantomData,
        })
    }
}

// TODO: Implement the ratchet key.
impl<Ciphertext: RatchetCiphertext, Payload: RatchetPayload<Ciphertext>>
    QueueRatchet<Ciphertext, Payload>
{
    /// Initialize a new ratchet key.
    pub fn random() -> Result<Self, RandomnessError> {
        let secret = RatchetSecret::random()?;
        secret
            .try_into()
            .map_err(|_| RandomnessError::InsufficientRandomness)
    }

    fn ratchet_forward(&mut self) -> Result<(), EncryptionError> {
        let secret = RatchetSecret::derive(&self.secret, Vec::new())
            .map_err(|_| EncryptionError::LibraryError)?;
        let key =
            RatchetKey::derive(&secret, Vec::new()).map_err(|_| EncryptionError::LibraryError)?;

        self.secret = secret;
        self.key = key;
        self.sequence_number += 1;

        Ok(())
    }

    /// Encrypt the given payload.
    pub fn encrypt(&mut self, payload: Payload) -> Result<QueueMessage, EncryptionError> {
        // TODO: We want domain separation: FQDN, UserID & ClientID.
        let ciphertext = payload.encrypt(&self.key)?;

        let queue_message = QueueMessage {
            sequence_number: self.sequence_number,
            ciphertext: ciphertext.as_ref().clone(),
        };

        self.ratchet_forward()?;

        Ok(queue_message)
    }

    /// Decrypt the given payload.
    pub fn decrypt(&mut self, queue_message: QueueMessage) -> Result<Payload, DecryptionError> {
        let ciphertext = queue_message.ciphertext.into();
        let plaintext = Payload::decrypt(&self.key, &ciphertext)?;
        self.ratchet_forward()
            .map_err(|_| DecryptionError::DecryptionError)?;
        Ok(plaintext)
    }

    /// Sample some fresh entropy and inject it into the current key. Returns the entropy.
    pub fn update(&mut self) -> RatchetKeyUpdate {
        todo!()
    }

    /// Get the current sequence number
    pub fn sequence_number(&self) -> u64 {
        self.sequence_number
    }

    pub fn secret(&self) -> &RatchetSecret {
        &self.secret
    }

    pub fn key(&self) -> &RatchetKey {
        &self.key
    }
}
