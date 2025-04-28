// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::marker::PhantomData;

use mls_assist::{
    openmls::prelude::SignaturePublicKey,
    openmls_rust_crypto::OpenMlsRustCrypto,
    openmls_traits::{OpenMlsProvider, crypto::OpenMlsCrypto},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::{
    LibraryError,
    crypto::{errors::KeyGenerationError, secrets::SecretBytes},
};

use super::{DEFAULT_SIGNATURE_SCHEME, signable::Signature};

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TlsSize, TlsSerialize, TlsDeserializeBytes,
)]
#[serde(transparent)]
pub struct VerifyingKey<KT> {
    key: Vec<u8>,
    _type: PhantomData<KT>,
}

pub struct VerifyingKeyRef<'a, KT> {
    key: &'a [u8],
    _type: PhantomData<KT>,
}

impl<KT, DB: sqlx::Database> sqlx::Type<DB> for VerifyingKey<KT>
where
    Vec<u8>: sqlx::Type<DB>,
{
    fn type_info() -> <DB as sqlx::Database>::TypeInfo {
        <Vec<u8> as sqlx::Type<DB>>::type_info()
    }
}

impl<'a, KT, DB: sqlx::Database> sqlx::Encode<'a, DB> for VerifyingKey<KT>
where
    Vec<u8>: sqlx::Encode<'a, DB>,
{
    fn encode_by_ref(
        &self,
        buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'a>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        <Vec<u8> as sqlx::Encode<DB>>::encode_by_ref(&self.key, buf)
    }
}

impl<'r, KT, DB: sqlx::Database> sqlx::Decode<'r, DB> for VerifyingKey<KT>
where
    Vec<u8>: sqlx::Decode<'r, DB>,
{
    fn decode(
        value: <DB as sqlx::Database>::ValueRef<'r>,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        <Vec<u8> as sqlx::Decode<DB>>::decode(value).map(Self::from_bytes)
    }
}

// We need these traits to interop the MLS leaf keys.
impl<'a, KT> From<&'a SignaturePublicKey> for VerifyingKeyRef<'a, KT> {
    fn from(pk: &'a SignaturePublicKey) -> Self {
        Self::from_slice(pk.as_slice())
    }
}

impl<KT> From<VerifyingKey<KT>> for SignaturePublicKey {
    fn from(pk: VerifyingKey<KT>) -> Self {
        SignaturePublicKey::from(pk.key)
    }
}

pub trait VerifyingKeyBehaviour {
    fn as_slice(&self) -> &[u8];

    /// Verify the given signature with the given payload. Returns an error if the
    /// verification fails or if the signature does not have the right length.
    fn verify(&self, payload: &[u8], signature: &[u8]) -> Result<(), SignatureVerificationError> {
        let rust_crypto = OpenMlsRustCrypto::default();
        rust_crypto
            .crypto()
            .verify_signature(
                DEFAULT_SIGNATURE_SCHEME,
                payload,
                self.as_slice(),
                signature,
            )
            .map_err(|_| SignatureVerificationError::VerificationFailure)
    }
}

impl<KT> VerifyingKeyBehaviour for &VerifyingKey<KT> {
    fn as_slice(&self) -> &[u8] {
        &self.key
    }
}

impl<KT> VerifyingKeyBehaviour for VerifyingKeyRef<'_, KT> {
    fn as_slice(&self) -> &[u8] {
        self.key
    }
}

impl<'a, KT> VerifyingKeyRef<'a, KT> {
    fn from_slice(key: &'a [u8]) -> Self {
        Self {
            key,
            _type: PhantomData,
        }
    }
}

impl<KT> VerifyingKey<KT> {
    #[cfg(any(test, feature = "test_utils"))]
    pub fn new_for_test(value: Vec<u8>) -> Self {
        Self::from_bytes(value)
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.key
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self {
            key: bytes,
            _type: PhantomData,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningKey<KT> {
    pub(super) signing_key: SecretBytes,
    pub(super) verifying_key: VerifyingKey<KT>,
}

impl<KT> SigningKey<KT> {
    pub fn generate() -> Result<SigningKey<KT>, KeyGenerationError> {
        let (private_key, public_key) = OpenMlsRustCrypto::default()
            .crypto()
            .signature_key_gen(DEFAULT_SIGNATURE_SCHEME)
            .map_err(|_| KeyGenerationError::KeypairGeneration)?;
        Ok(Self {
            signing_key: SecretBytes::from(private_key),
            verifying_key: VerifyingKey::from_bytes(public_key),
        })
    }

    pub fn verifying_key(&self) -> &VerifyingKey<KT> {
        &self.verifying_key
    }

    /// Sign the given payload with this signing key.
    pub fn sign(&self, payload: &[u8]) -> Result<Signature, LibraryError> {
        let rust_crypto = OpenMlsRustCrypto::default();
        rust_crypto
            .crypto()
            .sign(DEFAULT_SIGNATURE_SCHEME, payload, &self.signing_key)
            .map_err(|_| LibraryError)
            .map(Signature::from_bytes)
    }
}

pub trait Convertible<Target> {}

impl<Source> VerifyingKey<Source> {
    pub fn convert<Target>(self) -> VerifyingKey<Target>
    where
        Source: Convertible<Target>,
    {
        VerifyingKey::from_bytes(self.key)
    }
}

impl<Source> SigningKey<Source> {
    pub fn convert<Target>(self) -> SigningKey<Target>
    where
        Source: Convertible<Target>,
    {
        SigningKey {
            signing_key: self.signing_key,
            verifying_key: VerifyingKey::from_bytes(self.verifying_key.key),
        }
    }
}

/// Error verifying signature.
#[derive(Error, Debug)]
pub enum SignatureVerificationError {
    /// Could not verify this signature with the given payload.
    #[error("Could not verify this mac with the given payload.")]
    VerificationFailure,
    /// Unrecoverable implementation error
    #[error(transparent)]
    LibraryError(#[from] LibraryError),
}

pub const SIGNATURE_PUBLIC_KEY_SIZE: usize = 32;
