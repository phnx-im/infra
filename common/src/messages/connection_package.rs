// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    openmls::prelude::HashType, openmls_rust_crypto::RustCrypto,
    openmls_traits::crypto::OpenMlsCrypto,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tls_codec::{Serialize as _, TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::{credentials::keys::HandleSignature, crypto::ConnectionEncryptionKey};

pub use payload::{ConnectionPackageIn, ConnectionPackagePayload};

mod payload {
    use super::*;
    use crate::{
        credentials::keys::{HandleKeyType, HandleVerifyingKey},
        crypto::{
            ConnectionEncryptionKey,
            signatures::{
                private_keys::SignatureVerificationError,
                signable::{Signable, SignedStruct, Verifiable, VerifiedStruct},
            },
        },
        identifiers::UserHandleHash,
        messages::MlsInfraVersion,
        time::ExpirationData,
    };

    #[derive(Debug, Clone, PartialEq, Eq, TlsSerialize, TlsSize, Serialize, Deserialize)]
    pub struct ConnectionPackagePayload {
        pub user_handle_hash: UserHandleHash,
        pub protocol_version: MlsInfraVersion,
        pub encryption_key: ConnectionEncryptionKey,
        pub lifetime: ExpirationData,
        pub verifying_key: HandleVerifyingKey,
    }

    mod private_mod {
        #[derive(Default)]
        pub struct Seal;
    }

    impl Signable for ConnectionPackagePayload {
        type SignedOutput = ConnectionPackage;

        fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
            self.tls_serialize_detached()
        }

        fn label(&self) -> &str {
            "ConnectionPackage"
        }
    }

    impl SignedStruct<ConnectionPackagePayload, HandleKeyType> for ConnectionPackage {
        fn from_payload(payload: ConnectionPackagePayload, signature: HandleSignature) -> Self {
            Self {
                payload: payload,
                signature,
            }
        }
    }

    #[derive(Debug)]
    pub struct ConnectionPackageIn {
        payload: ConnectionPackagePayload,
        signature: HandleSignature,
    }

    impl ConnectionPackageIn {
        pub fn new(payload: ConnectionPackagePayload, signature: HandleSignature) -> Self {
            Self { payload, signature }
        }

        pub fn verify(self) -> Result<ConnectionPackage, SignatureVerificationError> {
            let verifying_key = self.payload.verifying_key.clone();
            <Self as Verifiable>::verify(self, &verifying_key)
        }
    }

    impl Verifiable for ConnectionPackageIn {
        fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
            self.payload.tls_serialize_detached()
        }

        fn signature(&self) -> impl AsRef<[u8]> {
            &self.signature
        }

        fn label(&self) -> &str {
            "ConnectionPackage"
        }
    }

    impl VerifiedStruct<ConnectionPackageIn> for ConnectionPackage {
        type SealingType = private_mod::Seal;

        fn from_verifiable(verifiable: ConnectionPackageIn, _seal: Self::SealingType) -> Self {
            ConnectionPackage {
                payload: verifiable.payload,
                signature: verifiable.signature,
            }
        }
    }
}

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, TlsSerialize, TlsSize, TlsDeserializeBytes,
)]
#[cfg_attr(any(feature = "test_utils", test), derive(PartialEq))]
#[serde(transparent)]
pub struct ConnectionPackageHash([u8; 32]);

#[derive(Debug, Error)]
pub enum ConnectionPackageHashError {
    #[error("Invalid length: expected 32 bytes, got {actual} bytes")]
    InvalidLength { actual: usize },
}

impl TryFrom<Vec<u8>> for ConnectionPackageHash {
    type Error = ConnectionPackageHashError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        let value_len = value.len();
        let array = value
            .try_into()
            .map_err(|_| ConnectionPackageHashError::InvalidLength { actual: value_len })?;
        Ok(Self(array))
    }
}

impl ConnectionPackageHash {
    pub fn to_bytes(self) -> [u8; 32] {
        self.0
    }

    #[cfg(feature = "test_utils")]
    pub fn random() -> Self {
        use rand::RngCore;

        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        Self(bytes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, TlsSerialize, TlsSize, Serialize, Deserialize)]
pub struct ConnectionPackage {
    payload: ConnectionPackagePayload,
    signature: HandleSignature,
}

impl ConnectionPackage {
    pub fn new(payload: ConnectionPackagePayload, signature: HandleSignature) -> Self {
        Self { payload, signature }
    }

    pub fn into_parts(self) -> (ConnectionPackagePayload, HandleSignature) {
        (self.payload, self.signature)
    }

    pub fn encryption_key(&self) -> &ConnectionEncryptionKey {
        &self.payload.encryption_key
    }

    pub fn hash(&self) -> ConnectionPackageHash {
        let rust_crypto = RustCrypto::default();
        let payload = self.tls_serialize_detached().unwrap_or_default();
        debug_assert!(!payload.is_empty());
        let input = [b"Connection Package".to_vec(), payload].concat();
        let value: [u8; 32] = rust_crypto
            .hash(HashType::Sha2_256, &input)
            .unwrap_or_default()
            .try_into()
            // Output length of `hash` is always 32 bytes
            .unwrap();
        debug_assert!(!value.is_empty());
        ConnectionPackageHash(value)
    }

    #[cfg(feature = "test_utils")]
    pub fn new_for_test(payload: ConnectionPackagePayload, signature: HandleSignature) -> Self {
        Self { payload, signature }
    }
}
