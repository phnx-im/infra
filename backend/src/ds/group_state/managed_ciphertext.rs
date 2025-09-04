// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(dead_code)]

use std::collections::HashMap;

use chrono::Duration;
use phnxcommon::{
    crypto::{
        ear::keys::GroupStateEarKeyType,
        errors::RandomnessError,
        indexed_aead::{
            ciphertexts::{
                IndexDecryptable, IndexDecryptionError, IndexEncryptable, IndexEncryptionError,
                IndexedCiphertext,
            },
            keys::{BaseSecret, Index, IndexedAeadKey, IndexedKeyType},
        },
    },
    time::{ExpirationData, TimeStamp},
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::EncryptedDsGroupStateCtype;

/// The time after which the ciphertext manager will issue a new key.
const INITIAL_KEY_VALIDITY: Duration = Duration::days(30);

/// The time that an old key is still valid after a new key has been issued.
/// This yields a total of 90 days of validity for any key.
const TOTAL_KEY_VALIDITY: Duration = Duration::days(90);

#[derive(Debug)]
pub struct GroupStateEarKeyCtype;

type KeyType = GroupStateEarKeyType;
type PayloadCtype = EncryptedDsGroupStateCtype;
type WrapperCtype = GroupStateEarKeyCtype;

#[derive(Debug, Serialize, Deserialize)]
struct WrappedKey<KeyType, WrapperCtype>
where
    KeyType: IndexedKeyType,
{
    #[serde(bound = "")]
    ciphertext: IndexedCiphertext<KeyType, WrapperCtype>,
    expiration: ExpirationData,
}

#[derive(Debug)]
pub struct BeforeDecryption;
#[derive(Debug)]
pub struct AfterDecryption<KeyType: IndexedKeyType>(IndexedAeadKey<KeyType>);
#[derive(Debug)]
pub struct AfterUpdate;

#[derive(Debug, Serialize, Deserialize)]
struct ManagedCiphertextInner<KeyType, PayloadCtype, WrapperCtype>
where
    KeyType: IndexedKeyType,
{
    #[serde(bound = "")]
    ciphertext: IndexedCiphertext<KeyType, PayloadCtype>,
    current_key_expiration: ExpirationData,
    // The main key encrypted under other keys
    #[serde(bound = "")]
    wrapped_keys: HashMap<Index<KeyType>, WrappedKey<KeyType, WrapperCtype>>,
}

/// A ciphertext that manages its own keys. It will keep track of keys and their
/// expiration dates. If it is decrypted and a key has expired, it will issue a
/// new key and wrap the old key. The old key will be valid for a certain amount
/// of time after the new key has been issued.
#[derive(Debug)]
pub struct ManagedCiphertext<KeyType, PayloadCtype, WrapperCtype, State>
where
    KeyType: IndexedKeyType,
{
    inner: ManagedCiphertextInner<KeyType, PayloadCtype, WrapperCtype>,
    /// The current key used to encrypt the payload. This is stored only
    /// temporarily between decrypting and updating the payload.
    state: State,
}

#[derive(Debug, Error)]
pub enum ManagedCiphertextError {
    #[error("Invalid decryption key")]
    InvalidDecryptionKey,
    #[error("Failed to generate a random key")]
    KeyGenerationFailure(#[from] RandomnessError),
    #[error("Failed to encrypt")]
    EncryptionFailure(#[from] IndexEncryptionError),
    #[error("Failed to decrypt")]
    DecryptionFailure(#[from] IndexDecryptionError),
    #[error("Library error")]
    LibraryError(#[from] phnxcommon::LibraryError),
}

#[derive(Debug)]
pub(crate) struct DecryptionResult<KeyType: IndexedKeyType, PayloadCtype, WrapperCtype, PayloadType>
{
    payload: PayloadType,
    new_key: Option<IndexedAeadKey<KeyType>>,
    managed_ciphertext:
        ManagedCiphertext<KeyType, PayloadCtype, WrapperCtype, AfterDecryption<KeyType>>,
}

impl<KeyType: std::fmt::Debug, PayloadCtype, WrapperCtype>
    ManagedCiphertext<KeyType, PayloadCtype, WrapperCtype, BeforeDecryption>
where
    KeyType: IndexedKeyType,
{
    fn flush_expired_keys(&mut self, now: impl Into<TimeStamp>) {
        let now = now.into();
        self.inner
            .wrapped_keys
            .retain(|_, wrapped_key| wrapped_key.expiration.validate_sans_io(now));
    }

    /// Issues a new key and resets the expiration time of the current key.
    fn issue_new_key(
        &mut self,
        now: impl Into<TimeStamp>,
        rng: &mut impl Rng,
        derivation_context: KeyType::DerivationContext<'_>,
    ) -> Result<IndexedAeadKey<KeyType>, ManagedCiphertextError> {
        self.inner.current_key_expiration =
            ExpirationData::new_sans_io(INITIAL_KEY_VALIDITY, now.into());
        derive_new_key(rng, derivation_context)
    }

    pub(crate) fn decrypt<PayloadType: IndexDecryptable<KeyType, PayloadCtype>>(
        mut self,
        now: impl Into<TimeStamp>,
        rng: &mut impl Rng,
        key: &IndexedAeadKey<KeyType>,
        derivation_context: KeyType::DerivationContext<'_>,
    ) -> Result<
        DecryptionResult<KeyType, PayloadCtype, WrapperCtype, PayloadType>,
        ManagedCiphertextError,
    >
    where
        BaseSecret<KeyType>:
            IndexEncryptable<KeyType, WrapperCtype> + IndexDecryptable<KeyType, WrapperCtype>,
    {
        let now = now.into();
        // Flush all expired keys
        self.flush_expired_keys(now);

        let current_key = {
            // Check if the key is the main key
            if key.index() == self.inner.ciphertext.key_index() {
                // Check if the key is still in the validity period
                let key_generated = self.inner.current_key_expiration.not_before();
                if *key_generated + TOTAL_KEY_VALIDITY < *now {
                    return Err(ManagedCiphertextError::InvalidDecryptionKey);
                }
                key.clone()
            } else {
                // If it is not, check if the key unwraps one of the wrapped
                // keys. If the unwrapped key is the main key, return it. If
                // not, look for the next wrapped key.
                let mut current_key = key.clone();
                loop {
                    let Some(wrapped_key) = self.inner.wrapped_keys.get(current_key.index()) else {
                        return Err(ManagedCiphertextError::InvalidDecryptionKey);
                    };
                    let unwrapped_key = IndexedAeadKey::decrypt_with_index(
                        &current_key,
                        &wrapped_key.ciphertext,
                        derivation_context.clone(),
                    )?;
                    current_key = unwrapped_key;
                    if current_key.index() == self.inner.ciphertext.key_index() {
                        break current_key;
                    }
                }
            }
        };

        // If it is, decrypt the payload
        let payload = PayloadType::decrypt_with_index(&current_key, &self.inner.ciphertext)?;

        // Check if the main key needs to be updated
        if self.inner.current_key_expiration.validate_sans_io(now) {
            // If it does not, return the decrypted payload
            let result = DecryptionResult {
                payload,
                new_key: None,
                managed_ciphertext: ManagedCiphertext {
                    inner: self.inner,
                    state: AfterDecryption(current_key),
                },
            };
            Ok(result)
        } else {
            // If it has, issue a new key, wrap the old key and return the payload
            let remaining_validity_period =
                *self.inner.current_key_expiration.not_before() + TOTAL_KEY_VALIDITY - *now;
            let new_key = self.issue_new_key(now, rng, derivation_context)?;
            println!("New key issued at {:?}", now);
            let ciphertext = new_key.encrypt_with_index(&current_key)?;
            let wrapped_key = WrappedKey {
                ciphertext,
                expiration: ExpirationData::new_sans_io(remaining_validity_period, now),
            };
            self.inner
                .wrapped_keys
                .insert(key.index().clone(), wrapped_key);
            let new_state = ManagedCiphertext::<_, _, _, AfterDecryption<KeyType>> {
                inner: self.inner,
                state: AfterDecryption(new_key.clone()),
            };
            let result = DecryptionResult {
                payload,
                new_key: Some(new_key),
                managed_ciphertext: new_state,
            };
            Ok(result)
        }
    }
}

impl<KeyType, PayloadCtype, WrapperCtype>
    ManagedCiphertext<KeyType, PayloadCtype, WrapperCtype, AfterDecryption<KeyType>>
where
    KeyType: IndexedKeyType,
{
    pub(crate) fn update_payload<PayloadType: IndexEncryptable<KeyType, PayloadCtype>>(
        mut self,
        payload: PayloadType,
    ) -> Result<
        ManagedCiphertext<KeyType, PayloadCtype, WrapperCtype, AfterUpdate>,
        ManagedCiphertextError,
    > {
        let AfterDecryption(key) = &self.state;
        // Encrypt the payload with the new key
        self.inner.ciphertext = payload.encrypt_with_index(key)?;
        let result = ManagedCiphertext {
            inner: self.inner,
            state: AfterUpdate,
        };
        Ok(result)
    }
}

impl<KeyType, PayloadCtype, WrapperCtype>
    ManagedCiphertext<KeyType, PayloadCtype, WrapperCtype, AfterUpdate>
where
    KeyType: IndexedKeyType,
{
    pub(crate) fn new<PayloadType: IndexEncryptable<KeyType, PayloadCtype>>(
        now: impl Into<TimeStamp>,
        rng: &mut impl Rng,
        payload: PayloadType,
        derivation_context: KeyType::DerivationContext<'_>,
    ) -> Result<(Self, IndexedAeadKey<KeyType>), ManagedCiphertextError> {
        let key = derive_new_key(rng, derivation_context)?;
        let current_key_expiration = ExpirationData::new_sans_io(INITIAL_KEY_VALIDITY, now.into());
        let ciphertext = payload.encrypt_with_index(&key)?;
        let inner = ManagedCiphertextInner {
            ciphertext,
            current_key_expiration,
            wrapped_keys: HashMap::new(),
        };
        let ctxt = ManagedCiphertext {
            inner,
            state: AfterUpdate,
        };
        Ok((ctxt, key))
    }
}

fn derive_new_key<KeyType: IndexedKeyType>(
    rng: &mut impl Rng,
    derivation_context: KeyType::DerivationContext<'_>,
) -> Result<IndexedAeadKey<KeyType>, ManagedCiphertextError> {
    let new_base_secret = BaseSecret::random_sans_io(rng)?;
    Ok(IndexedAeadKey::from_base_secret(
        new_base_secret,
        derivation_context,
    )?)
}

impl<KeyType: IndexedKeyType, PayloadCtype, WrapperCtype> serde::Serialize
    for ManagedCiphertext<KeyType, PayloadCtype, WrapperCtype, AfterUpdate>
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Don't serialize the state, i.e. the key
        self.inner.serialize(serializer)
    }
}

impl<'de, KeyType: IndexedKeyType, PayloadCtype, WrapperCtype> serde::Deserialize<'de>
    for ManagedCiphertext<KeyType, PayloadCtype, WrapperCtype, BeforeDecryption>
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Deserialize the inner struct and set the state to BeforeDecryption
        let inner = ManagedCiphertextInner::deserialize(deserializer)?;
        Ok(ManagedCiphertext {
            inner,
            state: BeforeDecryption,
        })
    }
}

#[cfg(test)]
mod tests {
    use phnxcommon::{
        codec::PhnxCodec,
        crypto::{
            ear::{EarDecryptable, EarEncryptable},
            indexed_aead::{
                ciphertexts::{IndexDecryptable, IndexEncryptable},
                keys::{BaseSecret, IndexedAeadKey, IndexedKeyType},
            },
        },
        time::{Duration, TimeStamp},
    };
    use serde::{Deserialize, Serialize};

    use crate::ds::group_state::managed_ciphertext::{DecryptionResult, ManagedCiphertextError};

    use super::{BeforeDecryption, ManagedCiphertext};

    #[derive(Debug, Serialize, Deserialize)]
    struct DummyPayload(Vec<u8>);
    #[derive(Debug)]
    struct DummyKeyType;
    type DummyKey = IndexedAeadKey<DummyKeyType>;
    type DummyKeyBaseSecret = BaseSecret<DummyKeyType>;

    #[derive(Debug)]
    struct DummyWrapperCtype;
    impl IndexedKeyType for DummyKeyType {
        type DerivationContext<'a> = &'a [u8];

        const LABEL: &'static str = "dummy_key_type";
    }

    impl EarEncryptable<DummyKey, DummyPayloadCtype> for DummyPayload {}
    impl EarDecryptable<DummyKey, DummyPayloadCtype> for DummyPayload {}

    impl EarEncryptable<DummyKey, DummyWrapperCtype> for DummyKeyBaseSecret {}
    impl EarDecryptable<DummyKey, DummyWrapperCtype> for DummyKeyBaseSecret {}

    impl IndexEncryptable<DummyKeyType, DummyWrapperCtype> for DummyKeyBaseSecret {}
    impl IndexDecryptable<DummyKeyType, DummyWrapperCtype> for DummyKeyBaseSecret {}
    #[derive(Debug)]
    struct DummyPayloadCtype;
    impl IndexEncryptable<DummyKeyType, DummyPayloadCtype> for DummyPayload {}
    impl IndexDecryptable<DummyKeyType, DummyPayloadCtype> for DummyPayload {}

    type DummyManagedCiphertext<State> =
        ManagedCiphertext<DummyKeyType, DummyPayloadCtype, DummyWrapperCtype, State>;

    #[test]
    fn encryption_decryption() {
        let now = TimeStamp::now();
        let rng = &mut rand::thread_rng();
        let derivation_context = vec![1u8; 32]; // Example derivation context

        let payload = DummyPayload(vec![1, 2, 3, 4, 5]);

        let (ciphertext, key) =
            DummyManagedCiphertext::<_>::new(now, rng, payload, &derivation_context).unwrap();

        // Serialize and deserialize the ciphertext
        let serialized = PhnxCodec::to_vec(&ciphertext).unwrap();
        let deserialized: DummyManagedCiphertext<BeforeDecryption> =
            PhnxCodec::from_slice(&serialized).unwrap();
        assert_eq!(deserialized.inner.ciphertext, ciphertext.inner.ciphertext);

        // Decrypt the ciphertext
        let decryption_result: DecryptionResult<
            DummyKeyType,
            DummyPayloadCtype,
            DummyWrapperCtype,
            DummyPayload,
        > = deserialized
            .decrypt(now, rng, &key, &derivation_context)
            .unwrap();
        assert_eq!(decryption_result.payload.0, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn payload_update() {
        let now = TimeStamp::now();
        let rng = &mut rand::thread_rng();
        let derivation_context = vec![1u8; 32]; // Example derivation context

        let payload = DummyPayload(vec![1, 2, 3, 4, 5]);

        let (ciphertext, key) =
            DummyManagedCiphertext::<_>::new(now, rng, payload, &derivation_context).unwrap();

        // Serialize and deserialize the ciphertext
        let serialized = PhnxCodec::to_vec(&ciphertext).unwrap();
        let deserialized: DummyManagedCiphertext<BeforeDecryption> =
            PhnxCodec::from_slice(&serialized).unwrap();
        assert_eq!(deserialized.inner.ciphertext, ciphertext.inner.ciphertext);

        // Decrypt the ciphertext
        let decryption_result: DecryptionResult<
            DummyKeyType,
            DummyPayloadCtype,
            DummyWrapperCtype,
            DummyPayload,
        > = deserialized
            .decrypt(now, rng, &key, &derivation_context)
            .unwrap();

        // Update the payload
        let updated_payload = DummyPayload(vec![6, 7, 8, 9, 10]);
        let updated_ciphertext = decryption_result
            .managed_ciphertext
            .update_payload(updated_payload)
            .unwrap();

        // Serialize the updated ciphertext
        let updated_serialized = PhnxCodec::to_vec(&updated_ciphertext).unwrap();
        // Deserialize the updated ciphertext
        let updated_deserialized: DummyManagedCiphertext<BeforeDecryption> =
            PhnxCodec::from_slice(&updated_serialized).unwrap();
        assert_eq!(
            updated_deserialized.inner.ciphertext,
            updated_ciphertext.inner.ciphertext
        );
        // Decrypt the updated ciphertext and verify the payload
        let updated_decryption_result: DecryptionResult<
            DummyKeyType,
            DummyPayloadCtype,
            DummyWrapperCtype,
            DummyPayload,
        > = updated_deserialized
            .decrypt(now, rng, &key, &derivation_context)
            .unwrap();
        assert_eq!(updated_decryption_result.payload.0, vec![6, 7, 8, 9, 10]);
    }

    #[test]
    fn key_wrapping() {
        let now = TimeStamp::now();
        let rng = &mut rand::thread_rng();
        let derivation_context = vec![1u8; 32]; // Example derivation context

        let payload = DummyPayload(vec![1, 2, 3, 4, 5]);

        let (ciphertext, key) =
            DummyManagedCiphertext::<_>::new(now, rng, payload, &derivation_context).unwrap();

        // Serialize and deserialize the ciphertext
        let serialized = PhnxCodec::to_vec(&ciphertext).unwrap();
        let deserialized: DummyManagedCiphertext<BeforeDecryption> =
            PhnxCodec::from_slice(&serialized).unwrap();
        assert_eq!(deserialized.inner.ciphertext, ciphertext.inner.ciphertext);

        // Now some time passes and we need to decrypt
        let now = *now + Duration::days(31); // Simulate time passing
        let decryption_result: DecryptionResult<
            DummyKeyType,
            DummyPayloadCtype,
            DummyWrapperCtype,
            DummyPayload,
        > = deserialized
            .decrypt(now, rng, &key, &derivation_context)
            .unwrap();
        // The payload should still be the same
        assert_eq!(decryption_result.payload.0, vec![1, 2, 3, 4, 5]);
        // A new key should have been issued
        assert!(decryption_result.new_key.is_some());
        // The key should be different from the original key
        let new_key = decryption_result.new_key.as_ref().unwrap();
        assert_ne!(new_key.index(), key.index());
        // Test if we can decrypt with both the old key and the new key
        let updated_ciphertext = decryption_result
            .managed_ciphertext
            .update_payload(decryption_result.payload)
            .unwrap();
        // Serialize the updated ciphertext
        let updated_serialized = PhnxCodec::to_vec(&updated_ciphertext).unwrap();
        // Deserialize the updated ciphertext
        let updated_deserialized: DummyManagedCiphertext<BeforeDecryption> =
            PhnxCodec::from_slice(&updated_serialized).unwrap();
        // Try with the old key
        let decryption_result_old_key: DecryptionResult<
            DummyKeyType,
            DummyPayloadCtype,
            DummyWrapperCtype,
            DummyPayload,
        > = updated_deserialized
            .decrypt(now, rng, &key, &derivation_context)
            .unwrap();
        assert_eq!(decryption_result_old_key.payload.0, vec![1, 2, 3, 4, 5]);

        // Try with the new key
        let updated_deserialized: DummyManagedCiphertext<BeforeDecryption> =
            PhnxCodec::from_slice(&updated_serialized).unwrap();
        let decryption_result_new_key: DecryptionResult<
            DummyKeyType,
            DummyPayloadCtype,
            DummyWrapperCtype,
            DummyPayload,
        > = updated_deserialized
            .decrypt(now, rng, new_key, &derivation_context)
            .unwrap();
        assert_eq!(decryption_result_new_key.payload.0, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn final_key_expiration() {
        let now = TimeStamp::now();
        let rng = &mut rand::thread_rng();
        let derivation_context = vec![1u8; 32]; // Example derivation context

        let payload = DummyPayload(vec![1, 2, 3, 4, 5]);

        let (ciphertext, key) =
            DummyManagedCiphertext::<_>::new(now, rng, payload, &derivation_context).unwrap();

        // Serialize and deserialize the ciphertext
        let serialized = PhnxCodec::to_vec(&ciphertext).unwrap();
        let deserialized: DummyManagedCiphertext<BeforeDecryption> =
            PhnxCodec::from_slice(&serialized).unwrap();
        assert_eq!(deserialized.inner.ciphertext, ciphertext.inner.ciphertext);

        // Now some time passes and we need to decrypt
        let now = *now + Duration::days(91); // Simulate time passing
        let decryption_result: ManagedCiphertextError = deserialized
            .decrypt::<DummyPayload>(now, rng, &key, &derivation_context)
            .unwrap_err();
        assert!(matches!(
            decryption_result,
            ManagedCiphertextError::InvalidDecryptionKey
        ));
    }

    #[test]
    fn full_life_cycle() {
        let now = TimeStamp::now();
        let rng = &mut rand::thread_rng();
        let derivation_context = vec![1u8; 32]; // Example derivation context

        let payload = DummyPayload(vec![1, 2, 3, 4, 5]);

        let (ciphertext, key) =
            DummyManagedCiphertext::<_>::new(now, rng, payload, &derivation_context).unwrap();

        // Serialize and deserialize the ciphertext
        let serialized = PhnxCodec::to_vec(&ciphertext).unwrap();
        let deserialized: DummyManagedCiphertext<BeforeDecryption> =
            PhnxCodec::from_slice(&serialized).unwrap();
        assert_eq!(deserialized.inner.ciphertext, ciphertext.inner.ciphertext);

        // Decrypt the ciphertext
        let ciphertext = deserialized;
        let decryption_result: DecryptionResult<
            DummyKeyType,
            DummyPayloadCtype,
            DummyWrapperCtype,
            DummyPayload,
        > = ciphertext
            .decrypt(now, rng, &key, &derivation_context)
            .unwrap();
        assert_eq!(decryption_result.payload.0, vec![1, 2, 3, 4, 5]);
        // There shouldn't be a new key issued yet
        assert!(decryption_result.new_key.is_none());

        // Update the payload
        let updated_payload = DummyPayload(vec![6, 7, 8, 9, 10]);
        let updated_ciphertext = decryption_result
            .managed_ciphertext
            .update_payload(updated_payload)
            .unwrap();
        // Serialize the updated ciphertext
        let updated_serialized = PhnxCodec::to_vec(&updated_ciphertext).unwrap();
        // Deserialize the updated ciphertext
        let updated_deserialized: DummyManagedCiphertext<BeforeDecryption> =
            PhnxCodec::from_slice(&updated_serialized).unwrap();
        assert_eq!(
            updated_deserialized.inner.ciphertext,
            updated_ciphertext.inner.ciphertext
        );

        // Now some time passes, the main key expires, and we need to decrypt again
        let now = *now + Duration::days(31); // Simulate time passing
        let decryption_result: DecryptionResult<
            DummyKeyType,
            DummyPayloadCtype,
            DummyWrapperCtype,
            DummyPayload,
        > = updated_deserialized
            .decrypt(now, rng, &key, &derivation_context)
            .unwrap();
        // The payload should still be the same
        assert_eq!(decryption_result.payload.0, vec![6, 7, 8, 9, 10]);
        // A new key should have been issued
        assert!(decryption_result.new_key.is_some());
        // The key should be different from the original key
        let new_key = decryption_result.new_key.as_ref().unwrap();
        assert_ne!(new_key.index(), key.index());
        // Update the ciphertext with the new key
        let updated_ciphertext = decryption_result
            .managed_ciphertext
            .update_payload(decryption_result.payload)
            .unwrap();
        // Serialize the updated ciphertext
        let updated_serialized = PhnxCodec::to_vec(&updated_ciphertext).unwrap();
        // Deserialize the updated ciphertext
        let updated_deserialized: DummyManagedCiphertext<BeforeDecryption> =
            PhnxCodec::from_slice(&updated_serialized).unwrap();
        println!("Old key index: {:?}", key.index());
        println!("New key index: {:?}", new_key.index());
        // We should now be able to decrypt the updated ciphertext with the new
        // key and the old key
        let _decryption_result_old_key: DecryptionResult<
            DummyKeyType,
            DummyPayloadCtype,
            DummyWrapperCtype,
            DummyPayload,
        > = updated_deserialized
            .decrypt(now, rng, &key, &derivation_context)
            .unwrap();

        let updated_deserialized: DummyManagedCiphertext<BeforeDecryption> =
            PhnxCodec::from_slice(&updated_serialized).unwrap();
        let _decryption_result_new_key: DecryptionResult<
            DummyKeyType,
            DummyPayloadCtype,
            DummyWrapperCtype,
            DummyPayload,
        > = updated_deserialized
            .decrypt(now, rng, new_key, &derivation_context)
            .unwrap();

        // Now some more time passes and the old key is out of the total validity
        let updated_deserialized: DummyManagedCiphertext<BeforeDecryption> =
            PhnxCodec::from_slice(&updated_serialized).unwrap();
        let now = now + Duration::days(61); // Simulate more time passing
        let decryption_result_old_key: ManagedCiphertextError = updated_deserialized
            .decrypt::<DummyPayload>(now, rng, &key, &derivation_context)
            .unwrap_err();
        assert!(matches!(
            decryption_result_old_key,
            ManagedCiphertextError::InvalidDecryptionKey
        ));
        // The new key should still be valid
        println!("Trying new key decryption after old key expiration");
        let updated_deserialized: DummyManagedCiphertext<BeforeDecryption> =
            PhnxCodec::from_slice(&updated_serialized).unwrap();
        let decryption_result_new_key: DecryptionResult<
            DummyKeyType,
            DummyPayloadCtype,
            DummyWrapperCtype,
            DummyPayload,
        > = updated_deserialized
            .decrypt(now, rng, new_key, &derivation_context)
            .unwrap();
        // A new key should have been issued again
        assert!(decryption_result_new_key.new_key.is_some());
        // Different from the previous new key
        let new_key2 = decryption_result_new_key.new_key.as_ref().unwrap();
        assert_ne!(new_key2.index(), new_key.index());
    }
}
