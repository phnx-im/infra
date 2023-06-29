use super::*;

#[cfg(test)]
mod tests;

#[derive(
    Serialize, PartialEq, Deserialize, Clone, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize,
)]
pub struct QueueRatchet<
    CiphertextType: AsRef<Ciphertext> + From<Ciphertext>,
    PayloadType: EarEncryptable<RatchetKey, CiphertextType> + EarDecryptable<RatchetKey, CiphertextType>,
> {
    sequence_number: u64,
    secret: RatchetSecret,
    key: RatchetKey,
    _phantom: PhantomData<(CiphertextType, PayloadType)>,
}

impl<
        CiphertextType: AsRef<Ciphertext> + From<Ciphertext>,
        PayloadType: EarEncryptable<RatchetKey, CiphertextType> + EarDecryptable<RatchetKey, CiphertextType>,
    > TryFrom<RatchetSecret> for QueueRatchet<CiphertextType, PayloadType>
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
impl<
        CiphertextType: AsRef<Ciphertext> + From<Ciphertext>,
        PayloadType: EarEncryptable<RatchetKey, CiphertextType> + EarDecryptable<RatchetKey, CiphertextType>,
    > QueueRatchet<CiphertextType, PayloadType>
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
        self.sequence_number = 0;

        Ok(())
    }

    /// Encrypt the given payload.
    pub fn encrypt(&mut self, payload: PayloadType) -> Result<QueueMessage, EncryptionError> {
        // TODO: We want domain separation: FQDN, UserID & ClientID.
        let ciphertext = payload.encrypt(&self.key)?;

        self.ratchet_forward()?;

        Ok(QueueMessage {
            sequence_number: self.sequence_number,
            ciphertext: ciphertext.as_ref().clone(),
        })
    }

    /// Decrypt the given payload.
    pub fn decrypt(&mut self, queue_message: QueueMessage) -> Result<PayloadType, DecryptionError> {
        let ciphertext = queue_message.ciphertext.into();
        let plaintext = PayloadType::decrypt(&self.key, &ciphertext)?;
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
}
