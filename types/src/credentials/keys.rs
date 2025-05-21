// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ops::Deref;

use mls_assist::{
    openmls::prelude::{
        BasicCredential, BasicCredentialError, Credential, Lifetime, OpenMlsProvider,
        SignatureScheme,
    },
    openmls_rust_crypto::OpenMlsRustCrypto,
    openmls_traits::{
        random::OpenMlsRand,
        signatures::{Signer, SignerError},
    },
};
use serde::{Deserialize, Serialize};
use sqlx::{Database, Decode, Encode, Sqlite, Type, encode::IsNull, error::BoxDynError};
use tls_codec::{
    DeserializeBytes as _, Serialize as _, TlsDeserializeBytes, TlsSerialize, TlsSize,
};
use tracing::error;

use crate::{
    codec::PhnxCodec,
    crypto::{RawKey, signatures::private_keys::Convertible},
};

use super::{AsCredential, AsIntermediateCredential};

use crate::crypto::{
    ear::{EarEncryptable, keys::IdentityLinkKey},
    errors::{EncryptionError, KeyGenerationError, RandomnessError},
    kdf::{KdfDerivable, keys::ConnectionKey},
    signatures::{
        DEFAULT_SIGNATURE_SCHEME,
        private_keys::{SigningKey, VerifyingKey},
        signable::Signable,
    },
};

use thiserror::Error;

use super::ClientCredential;

#[derive(Debug)]
pub struct AsIntermediateKeyType;

impl RawKey for AsIntermediateKeyType {}

#[derive(Clone, Serialize, Deserialize)]
pub struct AsIntermediateSigningKey {
    signing_key: SigningKey<AsIntermediateKeyType>,
    credential: AsIntermediateCredential,
}

impl Deref for AsIntermediateSigningKey {
    type Target = SigningKey<AsIntermediateKeyType>;

    fn deref(&self) -> &Self::Target {
        &self.signing_key
    }
}

impl Convertible<AsIntermediateKeyType> for PreliminaryAsKeyType {}

impl AsIntermediateSigningKey {
    pub fn from_prelim_key(
        prelim_key: PreliminaryAsIntermediateSigningKey,
        credential: AsIntermediateCredential,
    ) -> Result<Self, SigningKeyCreationError> {
        let prelim_key = prelim_key.convert();
        if prelim_key.verifying_key() != credential.verifying_key() {
            return Err(SigningKeyCreationError::PublicKeyMismatch);
        }
        Ok(Self {
            signing_key: prelim_key,
            credential,
        })
    }

    pub fn credential(&self) -> &AsIntermediateCredential {
        &self.credential
    }

    pub fn into_credential(self) -> AsIntermediateCredential {
        self.credential
    }
}

#[derive(Debug, Error)]
pub enum SigningKeyCreationError {
    #[error("Public key mismatch")]
    PublicKeyMismatch,
}

#[derive(Debug)]
pub struct AsKeyType;

impl RawKey for AsKeyType {}

#[derive(Debug, Serialize, Deserialize)]
pub struct AsSigningKey {
    signing_key: SigningKey<AsKeyType>,
    credential: AsCredential,
}

impl Deref for AsSigningKey {
    type Target = SigningKey<AsKeyType>;

    fn deref(&self) -> &Self::Target {
        &self.signing_key
    }
}

impl AsSigningKey {
    pub fn from_private_key_and_credential(
        private_key: SigningKey<AsKeyType>,
        credential: AsCredential,
    ) -> Self {
        Self {
            signing_key: private_key,
            credential,
        }
    }

    pub fn credential(&self) -> &AsCredential {
        &self.credential
    }

    pub fn into_credential(self) -> AsCredential {
        self.credential
    }
}

pub type AsVerifyingKey = VerifyingKey<AsKeyType>;

pub type AsIntermediateVerifyingKey = VerifyingKey<AsIntermediateKeyType>;

#[derive(Debug)]
pub struct ClientKeyType;

impl RawKey for ClientKeyType {}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClientSigningKey {
    signing_key: SigningKey<ClientKeyType>, // private
    credential: ClientCredential,           // known to other users and the server
}

impl TryFrom<&ClientCredential> for Credential {
    type Error = tls_codec::Error;

    fn try_from(value: &ClientCredential) -> Result<Self, Self::Error> {
        let basic_credential = BasicCredential::new(value.tls_serialize_detached()?);
        Ok(basic_credential.into())
    }
}

impl TryFrom<Credential> for ClientCredential {
    type Error = BasicCredentialError;

    fn try_from(value: Credential) -> Result<Self, Self::Error> {
        let basic_credential = BasicCredential::try_from(value)?;
        let credential =
            ClientCredential::tls_deserialize_exact_bytes(basic_credential.identity())?;
        Ok(credential)
    }
}

impl Type<Sqlite> for ClientSigningKey {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <Vec<u8> as Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for ClientSigningKey {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        let bytes = PhnxCodec::to_vec(self)?;
        Encode::<Sqlite>::encode(bytes, buf)
    }
}

impl<'r> Decode<'r, Sqlite> for ClientSigningKey {
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let bytes: &[u8] = Decode::<Sqlite>::decode(value)?;
        let value = PhnxCodec::from_slice(bytes)?;
        Ok(value)
    }
}

impl Deref for ClientSigningKey {
    type Target = SigningKey<ClientKeyType>;

    fn deref(&self) -> &Self::Target {
        &self.signing_key
    }
}

impl Convertible<ClientKeyType> for PreliminaryClientKeyType {}

impl ClientSigningKey {
    pub fn from_prelim_key(
        prelim_key: PreliminaryClientSigningKey,
        credential: ClientCredential,
    ) -> Result<Self, SigningKeyCreationError> {
        let prelim_key = prelim_key.convert();
        if prelim_key.verifying_key() != credential.verifying_key() {
            return Err(SigningKeyCreationError::PublicKeyMismatch);
        }
        Ok(Self {
            signing_key: prelim_key,
            credential,
        })
    }

    pub fn credential(&self) -> &ClientCredential {
        &self.credential
    }
}

pub type ClientVerifyingKey = VerifyingKey<ClientKeyType>;

// #[derive(Debug)]
// pub struct PseudonymousKeyType;

// #[derive(Clone, Debug, Serialize, Deserialize)]
// pub struct PseudonymousCredentialSigningKey {
//     signing_key: SigningKey<PseudonymousKeyType>,
//     credential: PseudonymousCredential,
// }

// impl Type<Sqlite> for PseudonymousCredentialSigningKey {
//     fn type_info() -> <Sqlite as Database>::TypeInfo {
//         <Vec<u8> as Type<Sqlite>>::type_info()
//     }
// }

// impl<'q> Encode<'q, Sqlite> for PseudonymousCredentialSigningKey {
//     fn encode_by_ref(
//         &self,
//         buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
//     ) -> Result<IsNull, BoxDynError> {
//         let bytes = PhnxCodec::to_vec(self)?;
//         Encode::<Sqlite>::encode(bytes, buf)
//     }
// }

// impl<'r> Decode<'r, Sqlite> for PseudonymousCredentialSigningKey {
//     fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
//         let bytes: &[u8] = Decode::<Sqlite>::decode(value)?;
//         let value = PhnxCodec::from_slice(bytes)?;
//         Ok(value)
//     }
// }

// 30 days lifetime in seconds
pub(crate) const DEFAULT_INFRA_CREDENTIAL_LIFETIME: u64 = 30 * 24 * 60 * 60;

#[derive(Debug, Error)]
pub enum CredentialCreationError {
    #[error(transparent)]
    KeyGenerationError(#[from] KeyGenerationError),
    #[error(transparent)]
    RandomnessError(#[from] RandomnessError),
    #[error("Failed to derive identity link key")]
    KeyDerivationFailed,
    #[error(transparent)]
    EncryptionFailed(#[from] EncryptionError),
}

/*
impl PseudonymousCredentialSigningKey {
    pub fn generate(
        client_signer: &ClientSigningKey,
        connection_key: &ConnectionKey,
    ) -> Result<(Self, IdentityLinkKey), CredentialCreationError> {
        // Construct the TBS
        let signing_key = SigningKey::generate()?;
        let identity = OpenMlsRustCrypto::default()
            .rand()
            .random_vec(32)
            .map_err(|_| RandomnessError::InsufficientRandomness)?;
        let tbs = PseudonymousCredentialTbs {
            identity,
            expiration_data: Lifetime::new(DEFAULT_INFRA_CREDENTIAL_LIFETIME),
            signature_scheme: DEFAULT_SIGNATURE_SCHEME,
            verifying_key: signing_key.verifying_key().clone().into(),
        };

        // Derive the identity link key based on the TBS
        let identity_link_key = IdentityLinkKey::derive(connection_key, &tbs).map_err(|e| {
            error!(%e, "Failed to derive identity link key");
            CredentialCreationError::KeyDerivationFailed
        })?;

        // Sign the TBS and encrypt the identity link
        let signed_pseudonymous_credential = tbs.sign(client_signer).unwrap();
        let encrypted_signature = signed_pseudonymous_credential
            .signature
            .encrypt(&identity_link_key)?;
        let encrypted_client_credential = client_signer.credential().encrypt(&identity_link_key)?;
        let identity_link_ctxt = IdentityLinkCtxt {
            encrypted_signature,
            encrypted_client_credential,
        };
        let credential = PseudonymousCredential::new(
            signed_pseudonymous_credential.payload.identity,
            signed_pseudonymous_credential.payload.expiration_data,
            signed_pseudonymous_credential.payload.signature_scheme,
            signed_pseudonymous_credential.payload.verifying_key,
            identity_link_ctxt,
        );
        let credential = Self {
            signing_key,
            credential,
        };
        Ok((credential, identity_link_key))
    }

    pub fn credential(&self) -> &PseudonymousCredential {
        &self.credential
    }
}

impl Deref for PseudonymousCredentialSigningKey {
    type Target = SigningKey<PseudonymousKeyType>;

    fn deref(&self) -> &Self::Target {
        &self.signing_key
    }
}
*/

impl Signer for ClientSigningKey {
    fn sign(&self, payload: &[u8]) -> Result<Vec<u8>, SignerError> {
        self.signing_key
            .sign(payload)
            .map_err(|_| SignerError::SigningError)
            .map(|s| s.into_bytes())
    }

    fn signature_scheme(&self) -> SignatureScheme {
        self.credential.signature_scheme()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PreliminaryClientKeyType;
pub type PreliminaryClientSigningKey = SigningKey<PreliminaryClientKeyType>;

#[derive(Debug, Clone, TlsDeserializeBytes, TlsSerialize, TlsSize, Serialize, Deserialize)]
pub struct PreliminaryAsKeyType;
pub type PreliminaryAsIntermediateSigningKey = SigningKey<PreliminaryAsKeyType>;

pub type PreliminaryAsIntermediateVerifyingKey = VerifyingKey<PreliminaryAsKeyType>;
