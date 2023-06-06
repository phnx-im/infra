// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Copied from OpenMLS
//!
//! This module defines traits used for signing and verifying
//! structs.
//!
//! # Type-Enforced Verification
//!
//! This module contains four traits, each describing the property they enable
//! upon implementation: [`Signable`], [`SignedStruct`], [`Verifiable`] and
//! [`VerifiedStruct`].
//!
//! Each trait represents the state of a struct in a sender-receiver flow with
//! the following transitions.
//!
//! * the signer creates an instance of a struct that implements [`Signable`]
//! * the signer signs it, consuming the [`Signable`] struct and producing a [`SignedStruct`]
//! * the signer serializes the struct and sends it to the verifier
//! * the verifier deserializes the byte-string into a struct implementing [`Verifiable`]
//! * the verifier verifies the struct, consuming the [`Verifiable`] struct and producing a [`VerifiedStruct`]
//!
//! Using this process, we can ensure that only structs implementing
//! [`SignedStruct`] are sent over the wire and only structs implementing
//! [`VerifiedStruct`] are used on the verifier side as input for further
//! processing functions.
//!
//! For the type-safety to work, it is important that [`Signable`] and
//! [`SignedStruct`] are implemented by distinct structs. The same goes for
//! [`Verifiable`] and [`VerifiedStruct`]. In addition, only the
//! [`SignedStruct`] should implement the [`tls_codec::Serialize`] trait.
//! Similarly, only the [`Verifiable`] struct should implement the
//! [`tls_codec::Deserialize`] trait.

use serde::{Deserialize, Serialize};
use tls_codec::{Serialize as TlsSerializeTrait, TlsDeserialize, TlsSerialize, TlsSize};
use utoipa::ToSchema;

use crate::{
    crypto::ear::{keys::SignatureEarKey, Ciphertext, EarDecryptable, EarEncryptable},
    messages::FriendshipToken,
    LibraryError,
};

use super::traits::{SignatureVerificationError, SigningKey, VerifyingKey};

pub type SignatureType = ed25519::Signature;

#[derive(Debug, Clone, ToSchema, TlsDeserialize, TlsSerialize, TlsSize, Serialize, Deserialize)]
pub struct Signature {
    signature: Vec<u8>,
}

impl Signature {
    pub(crate) fn as_slice(&self) -> &[u8] {
        &self.signature
    }

    pub(super) fn from_bytes(bytes: Vec<u8>) -> Self {
        Self { signature: bytes }
    }

    pub(crate) fn from_token(token: FriendshipToken) -> Self {
        Self {
            signature: token.token().to_vec(),
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.signature
    }
}

#[derive(Clone, Debug, TlsSerialize, TlsDeserialize, TlsSize, Serialize, Deserialize)]
pub struct EncryptedSignature {
    ciphertext: Ciphertext,
}

impl From<Ciphertext> for EncryptedSignature {
    fn from(ciphertext: Ciphertext) -> Self {
        Self { ciphertext }
    }
}

impl AsRef<Ciphertext> for EncryptedSignature {
    fn as_ref(&self) -> &Ciphertext {
        &self.ciphertext
    }
}

impl EarEncryptable<SignatureEarKey, EncryptedSignature> for Signature {}
impl EarDecryptable<SignatureEarKey, EncryptedSignature> for Signature {}

/// This trait must be implemented by all structs that contain a self-signature.
pub trait SignedStruct<T> {
    /// Build a signed struct version from the payload struct.
    fn from_payload(payload: T, signature: Signature) -> Self;
}

/// Labeled signature content.
#[derive(Debug, Clone, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct SignContent {
    label: Vec<u8>,
    content: Vec<u8>,
}

const SIGN_LABEL_PREFIX: &str = "Phoenix Homeserver Protocol 1.0";

impl From<(&str, &[u8])> for SignContent {
    fn from((label, content): (&str, &[u8])) -> Self {
        let label_string = SIGN_LABEL_PREFIX.to_owned() + label;
        let label = label_string.as_bytes().into();
        Self {
            label,
            content: content.into(),
        }
    }
}

pub enum SigningError {
    SerializationError,
}

/// This trait must be implemented by all structs that contain a verified
/// self-signature.
pub trait VerifiedStruct<T> {
    /// This type is used to prevent users of the trait from bypassing `verify`
    /// by simply calling `from_verifiable`. `Seal` should be a dummy type
    /// defined in a private module as follows:
    /// ```
    /// mod private_mod {
    ///     pub struct Seal;
    ///
    ///     impl Default for Seal {
    ///         fn default() -> Self {
    ///             Seal {}
    ///         }
    ///     }
    /// }
    /// ```
    type SealingType: Default;

    /// Build a verified struct version from the payload struct. This function
    /// is only meant to be called by the implementation of the `Verifiable`
    /// trait corresponding to this `VerifiedStruct`.
    #[doc(hidden)]
    fn from_verifiable(verifiable: T, _seal: Self::SealingType) -> Self;
}

/// The `Signable` trait is implemented by all struct that are being signed.
/// The implementation has to provide the `unsigned_payload` function.
pub trait Signable: Sized {
    /// The type of the object once it's signed.
    type SignedOutput;

    /// Return the unsigned, serialized payload that should be signed.
    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error>;

    /// Return the string label used for labeled signing.
    fn label(&self) -> &str;

    /// Sign the payload.
    ///
    /// Returns a `Signature`.
    fn sign(self, signing_key: &impl SigningKey) -> Result<Self::SignedOutput, LibraryError>
    where
        Self::SignedOutput: SignedStruct<Self>,
    {
        let payload = self
            .unsigned_payload()
            .map_err(LibraryError::missing_bound_check)?;
        let sign_content: SignContent = (self.label(), payload.as_slice()).into();
        let signature = signing_key.sign(
            &sign_content
                .tls_serialize_detached()
                .map_err(LibraryError::missing_bound_check)?,
        )?;
        Ok(Self::SignedOutput::from_payload(self, signature))
    }
}

/// The verifiable trait must be implemented by any struct that is signed with
/// a credential. The actual `verify` method is provided.
/// The `unsigned_payload` and `signature` functions have to be implemented for
/// each struct, returning the serialized payload and the signature respectively.
///
/// Note that `Verifiable` should not be implemented on the same struct as
/// `Signable`. If this appears to be necessary, it is probably a sign that the
/// struct implementing them aren't well defined. Not that both traits define an
/// `unsigned_payload` function.
pub trait Verifiable: Sized {
    /// Return the unsigned, serialized payload that should be verified.
    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error>;

    /// A reference to the signature to be verified.
    fn signature(&self) -> &Signature;

    /// Return the string label used for labeled verification.
    fn label() -> &'static str;

    /// Verifies the payload against the given `credential`.
    /// The signature is fetched via the [`Verifiable::signature()`] function and
    /// the payload via [`Verifiable::unsigned_payload()`].
    ///
    /// Returns `Ok(Self::VerifiedOutput)` if the signature is valid and
    /// `CredentialError::InvalidSignature` otherwise.
    fn verify<T>(
        self,
        signature_public_key: &impl VerifyingKey,
    ) -> Result<T, SignatureVerificationError>
    where
        T: VerifiedStruct<Self>,
    {
        let payload = self
            .unsigned_payload()
            .map_err(LibraryError::missing_bound_check)?;
        let sign_content: SignContent = (Self::label(), payload.as_slice()).into();
        let serialized_sign_content = sign_content
            .tls_serialize_detached()
            .map_err(LibraryError::missing_bound_check)?;
        signature_public_key.verify(&serialized_sign_content, self.signature())?;
        Ok(T::from_verifiable(self, T::SealingType::default()))
    }
}
