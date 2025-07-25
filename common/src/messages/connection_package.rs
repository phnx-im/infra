// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use chrono::Duration;
use mls_assist::{
    openmls::prelude::HashType, openmls_rust_crypto::RustCrypto,
    openmls_traits::crypto::OpenMlsCrypto,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tls_codec::{Serialize as _, TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::{
    LibraryError,
    credentials::keys::{HandleSignature, HandleSigningKey},
    crypto::{
        ConnectionDecryptionKey, ConnectionEncryptionKey, errors::RandomnessError,
        signatures::signable::Signable,
    },
    identifiers::UserHandleHash,
    messages::MlsInfraVersion,
    time::{ExpirationData, TimeStamp},
};

pub use payload::{ConnectionPackageIn, ConnectionPackagePayload};

pub(crate) const CONNECTION_PACKAGE_EXPIRATION: Duration = Duration::days(30);

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
        pub protocol_version: MlsInfraVersion,
        pub user_handle_hash: UserHandleHash,
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
            Self { payload, signature }
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

#[derive(Debug, Error)]
pub enum ConnectionPackageError {
    #[error(transparent)]
    LibraryError(#[from] LibraryError),
    #[error("Error generating decryption key: {0}")]
    DecryptionKeyError(#[from] RandomnessError),
}

impl ConnectionPackage {
    pub fn new(
        user_handle_hash: UserHandleHash,
        signing_key: &HandleSigningKey,
    ) -> Result<(ConnectionDecryptionKey, Self), ConnectionPackageError> {
        let decryption_key = ConnectionDecryptionKey::generate()?;
        let payload = ConnectionPackagePayload {
            protocol_version: MlsInfraVersion::default(),
            user_handle_hash,
            encryption_key: decryption_key.encryption_key().clone(),
            lifetime: ExpirationData::new(CONNECTION_PACKAGE_EXPIRATION),
            verifying_key: signing_key.verifying_key().clone(),
        };
        let connection_package = payload.sign(signing_key)?;
        Ok((decryption_key, connection_package))
    }

    pub fn from_parts(payload: ConnectionPackagePayload, signature: HandleSignature) -> Self {
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
        ConnectionPackageHash(value)
    }

    pub fn expires_at(&self) -> TimeStamp {
        self.payload.lifetime.not_after()
    }

    #[cfg(feature = "test_utils")]
    pub fn new_for_test(payload: ConnectionPackagePayload, signature: HandleSignature) -> Self {
        Self { payload, signature }
    }
}

mod sqlx_impls {
    use sqlx::{Database, Decode, Encode, Sqlite, Type, error::BoxDynError};

    use super::*;

    impl Type<Sqlite> for ConnectionPackageHash {
        fn type_info() -> <Sqlite as Database>::TypeInfo {
            <Vec<u8> as Type<Sqlite>>::type_info()
        }
    }

    impl Encode<'_, Sqlite> for ConnectionPackageHash {
        fn encode_by_ref(
            &self,
            buf: &mut <Sqlite as Database>::ArgumentBuffer<'_>,
        ) -> Result<sqlx::encode::IsNull, BoxDynError> {
            let bytes = self.to_bytes().to_vec();
            Encode::<Sqlite>::encode(bytes, buf)
        }
    }

    impl Decode<'_, Sqlite> for ConnectionPackageHash {
        fn decode(value: <Sqlite as Database>::ValueRef<'_>) -> Result<Self, BoxDynError> {
            let bytes: &[u8] = Decode::<Sqlite>::decode(value)?;
            Ok(Self(bytes.try_into()?))
        }
    }
}
pub mod legacy {
    use super::*;

    use crate::{
        credentials::{ClientCredential, keys::ClientSignature},
        messages::MlsInfraVersion,
        time::ExpirationData,
    };

    #[derive(Debug, Clone, PartialEq, Eq, TlsSerialize, TlsSize, Serialize, Deserialize)]
    pub struct ConnectionPackagePayloadV1 {
        pub protocol_version: MlsInfraVersion,
        pub encryption_key: ConnectionEncryptionKey,
        pub lifetime: ExpirationData,
        pub client_credential: ClientCredential,
    }

    #[derive(Debug, Clone, PartialEq, Eq, TlsSerialize, TlsSize, Serialize, Deserialize)]
    pub struct ConnectionPackageV1 {
        payload: ConnectionPackagePayloadV1,
        signature: ClientSignature,
    }
}
