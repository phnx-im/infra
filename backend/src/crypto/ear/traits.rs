//! This module contains traits to facilitate EAR of other structs on the
//! backend. Any struct that needs to be encrypted at rest needs to implement
//! the [`EarEncryptable`] trait.

use aes_gcm::{
    aead::{Aead as AesGcmAead, Key, Nonce},
    NewAead,
};
use serde::{de::DeserializeOwned, Serialize};
use tracing::instrument;

use crate::crypto::{secrets::Secret, RandomnessError};

use super::{Aead, Ciphertext, AEAD_KEY_SIZE, AEAD_NONCE_SIZE};

/// Errors that can occur during an encryption operation.
pub enum EncryptionError {
    RandomnessError, // Not enough randomness to generate Nonce
    LibraryError,    // Error encrypting the plaintext
}

/// Errors that can occur during a decryption operation.
pub enum DecryptionError {
    DecryptionError, // Error decrypting the ciphertext
}

/// A trait meant for structs holding a symmetric key of size [`AEAD_KEY_SIZE`].
/// It enables use of these keys for encryption and decryption operations.
pub trait EarKey: AsRef<Secret<AEAD_KEY_SIZE>> + From<Secret<AEAD_KEY_SIZE>> {
    // Encrypt the given plaintext under the given key. Generates a random nonce internally.
    #[instrument(level = "trace", skip_all, fields(key_type = std::any::type_name::<Self>()))]
    fn encrypt(&self, plaintext: &[u8]) -> Result<Ciphertext, EncryptionError> {
        // TODO: from_slice can potentially panic. However, we can rule this out
        // with a single test, since both the AEAD algorithm and the key size
        // are static.
        let key = Key::<Aead>::from_slice(self.as_ref().secret());
        let cipher: Aead = Aead::new(key);
        // TODO: Use a proper RNG provider instead.
        let nonce_raw = Secret::<AEAD_NONCE_SIZE>::random().map_err(|e| match e {
            RandomnessError::InsufficientRandomness => EncryptionError::RandomnessError,
        })?;
        let nonce = Nonce::<Aead>::from(nonce_raw.secret);
        // The Aead trait surfaces an error, but it's not clear under which
        // circumstances it would actually fail.
        let ciphertext = cipher
            .encrypt(&nonce, plaintext)
            .map_err(|_| EncryptionError::LibraryError)?;
        Ok(Ciphertext {
            ciphertext,
            nonce: nonce.into(),
        })
    }

    // Decrypt the given ciphertext (including the nonce) using the given key.
    #[instrument(level = "trace", skip_all, fields(key_type = std::any::type_name::<Self>()))]
    fn decrypt(&self, ciphertext: &Ciphertext) -> Result<Vec<u8>, DecryptionError> {
        // TODO: from_slice can potentially panic. However, we can rule this out
        // with a single test, since both the AEAD algorithm and the key size
        // are static.
        let key = Key::<Aead>::from_slice(self.as_ref().secret());
        let cipher: Aead = Aead::new(key);
        // TODO: Use a proper RNG provider instead.
        cipher
            .decrypt(&ciphertext.nonce.into(), ciphertext.ciphertext.as_slice())
            .map_err(|_| DecryptionError::DecryptionError)
    }
}

/// A trait that can be derived for structs that are encryptable/decryptable by
/// an EAR key.
pub trait EarEncryptable<EarKeyType: EarKey, CiphertextType: AsRef<Ciphertext> + From<Ciphertext>>:
    Serialize + DeserializeOwned
{
    /// Encrypt the value under the given [`EarKey`]. Returns an
    /// [`EncryptionError`] or the ciphertext.
    fn encrypt(&self, ear_key: &EarKeyType) -> Result<CiphertextType, EncryptionError> {
        let plaintext = serde_json::to_vec(self).map_err(|_| EncryptionError::LibraryError)?;
        let ciphertext = ear_key.encrypt(&plaintext)?;
        Ok(ciphertext.into())
    }

    /// Decrypt the given ciphertext using the given [`EarKey`]. Returns a
    /// [`DecryptionError`] or the resulting plaintext.
    fn decrypt(ear_key: &EarKeyType, ciphertext: &CiphertextType) -> Result<Self, DecryptionError> {
        let ciphertext = ciphertext.as_ref();
        let plaintext = ear_key.decrypt(ciphertext)?;
        let res =
            serde_json::from_slice(&plaintext).map_err(|_| DecryptionError::DecryptionError)?;
        Ok(res)
    }
}
