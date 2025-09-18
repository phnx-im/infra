// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tls_codec::{Serialize as _, TlsSerialize, TlsSize};

use crate::{
    LibraryError,
    credentials::keys::{HandleSignature, HandleSigningKey},
    crypto::{
        ConnectionDecryptionKey, ConnectionEncryptionKey, Labeled,
        errors::RandomnessError,
        hash::{Hash, Hashable},
        signatures::signable::Signable,
    },
    identifiers::UserHandleHash,
    messages::{
        AirProtocolVersion,
        connection_package::{CONNECTION_PACKAGE_EXPIRATION, ConnectionPackage},
        connection_package_v2::payload::TlsBool,
    },
    time::{ExpirationData, TimeStamp},
};

pub use payload::{ConnectionPackageV2In, ConnectionPackageV2Payload};

const LABEL: &str = "ConnectionPackageV2";

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
        messages::AirProtocolVersion,
        time::ExpirationData,
    };

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct TlsBool(pub bool);

    impl From<bool> for TlsBool {
        fn from(value: bool) -> Self {
            TlsBool(value)
        }
    }

    impl tls_codec::Size for TlsBool {
        fn tls_serialized_len(&self) -> usize {
            1
        }
    }

    impl tls_codec::Serialize for TlsBool {
        fn tls_serialize<W: std::io::Write>(
            &self,
            writer: &mut W,
        ) -> Result<usize, tls_codec::Error> {
            let byte = if self.0 { 1u8 } else { 0u8 };
            writer.write_all(&[byte])?;
            Ok(1)
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, TlsSerialize, TlsSize, Serialize, Deserialize)]
    pub struct ConnectionPackageV2Payload {
        pub protocol_version: AirProtocolVersion,
        pub user_handle_hash: UserHandleHash,
        pub encryption_key: ConnectionEncryptionKey,
        pub lifetime: ExpirationData,
        pub verifying_key: HandleVerifyingKey,
        pub is_last_resort: TlsBool,
    }

    mod private_mod {
        #[derive(Default)]
        pub struct Seal;
    }

    impl Signable for ConnectionPackageV2Payload {
        type SignedOutput = ConnectionPackageV2;

        fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
            self.tls_serialize_detached()
        }

        fn label(&self) -> &str {
            LABEL
        }
    }

    impl SignedStruct<ConnectionPackageV2Payload, HandleKeyType> for ConnectionPackageV2 {
        fn from_payload(payload: ConnectionPackageV2Payload, signature: HandleSignature) -> Self {
            Self { payload, signature }
        }
    }

    #[derive(Debug)]
    pub struct ConnectionPackageV2In {
        payload: ConnectionPackageV2Payload,
        signature: HandleSignature,
    }

    impl ConnectionPackageV2In {
        pub fn new(payload: ConnectionPackageV2Payload, signature: HandleSignature) -> Self {
            Self { payload, signature }
        }

        pub fn verify(self) -> Result<ConnectionPackageV2, SignatureVerificationError> {
            let verifying_key = self.payload.verifying_key.clone();
            <Self as Verifiable>::verify(self, &verifying_key)
        }
    }

    impl Verifiable for ConnectionPackageV2In {
        fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
            self.payload.tls_serialize_detached()
        }

        fn signature(&self) -> impl AsRef<[u8]> {
            &self.signature
        }

        fn label(&self) -> &str {
            LABEL
        }
    }

    impl VerifiedStruct<ConnectionPackageV2In> for ConnectionPackageV2 {
        type SealingType = private_mod::Seal;

        fn from_verifiable(verifiable: ConnectionPackageV2In, _seal: Self::SealingType) -> Self {
            ConnectionPackageV2 {
                payload: verifiable.payload,
                signature: verifiable.signature,
            }
        }
    }
}

impl Labeled for ConnectionPackageV2 {
    const LABEL: &'static str = LABEL;
}

pub type ConnectionPackageV2Hash = Hash<ConnectionPackageV2>;

impl Hashable for ConnectionPackageV2 {}

#[derive(Debug, Clone, PartialEq, Eq, TlsSerialize, TlsSize, Serialize, Deserialize)]
pub struct ConnectionPackageV2 {
    payload: ConnectionPackageV2Payload,
    signature: HandleSignature,
}

#[derive(Debug, Error)]
pub enum ConnectionPackageError {
    #[error(transparent)]
    LibraryError(#[from] LibraryError),
    #[error("Error generating decryption key: {0}")]
    DecryptionKeyError(#[from] RandomnessError),
}

impl ConnectionPackageV2 {
    pub fn new(
        user_handle_hash: UserHandleHash,
        signing_key: &HandleSigningKey,
        is_last_resort: bool,
    ) -> Result<(ConnectionDecryptionKey, Self), ConnectionPackageError> {
        let decryption_key = ConnectionDecryptionKey::generate()?;
        let payload = ConnectionPackageV2Payload {
            protocol_version: AirProtocolVersion::default(),
            user_handle_hash,
            encryption_key: decryption_key.encryption_key().clone(),
            lifetime: ExpirationData::new(CONNECTION_PACKAGE_EXPIRATION),
            verifying_key: signing_key.verifying_key().clone(),
            is_last_resort: TlsBool(is_last_resort),
        };
        let connection_package = payload.sign(signing_key)?;
        Ok((decryption_key, connection_package))
    }

    pub fn from_parts(payload: ConnectionPackageV2Payload, signature: HandleSignature) -> Self {
        Self { payload, signature }
    }

    pub fn into_parts(self) -> (ConnectionPackageV2Payload, HandleSignature) {
        (self.payload, self.signature)
    }

    pub fn encryption_key(&self) -> &ConnectionEncryptionKey {
        &self.payload.encryption_key
    }

    pub fn expires_at(&self) -> TimeStamp {
        self.payload.lifetime.not_after()
    }

    pub fn is_last_resort(&self) -> bool {
        self.payload.is_last_resort.0
    }

    #[cfg(feature = "test_utils")]
    pub fn new_for_test(payload: ConnectionPackageV2Payload, signature: HandleSignature) -> Self {
        Self { payload, signature }
    }
}

impl From<ConnectionPackage> for ConnectionPackageV2 {
    fn from(v1: ConnectionPackage) -> Self {
        let (payload, signature) = v1.into_parts();
        let payload = ConnectionPackageV2Payload {
            protocol_version: payload.protocol_version,
            user_handle_hash: payload.user_handle_hash,
            encryption_key: payload.encryption_key,
            lifetime: payload.lifetime,
            verifying_key: payload.verifying_key,
            is_last_resort: false.into(),
        };
        Self { payload, signature }
    }
}
