// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    openmls::prelude::SignaturePublicKey,
    openmls_rust_crypto::OpenMlsRustCrypto,
    openmls_traits::{crypto::OpenMlsCrypto, OpenMlsProvider},
};
use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::crypto::{errors::KeyGenerationError, secrets::SecretBytes};

use super::DEFAULT_SIGNATURE_SCHEME;

#[derive(
    Debug, Clone, Serialize, Deserialize, TlsSerialize, TlsDeserializeBytes, TlsSize, PartialEq, Eq,
)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
pub struct VerifyingKey(Vec<u8>);

// We need these traits to interop the MLS leaf keys.
impl From<SignaturePublicKey> for VerifyingKey {
    fn from(pk: SignaturePublicKey) -> Self {
        Self(pk.as_slice().to_vec())
    }
}

impl From<VerifyingKey> for SignaturePublicKey {
    fn from(pk: VerifyingKey) -> Self {
        SignaturePublicKey::from(pk.0)
    }
}

impl VerifyingKey {
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(
    feature = "sqlx",
    derive(sqlx::Type),
    sqlx(type_name = "signing_key_data")
)]
pub struct SigningKey {
    signing_key: SecretBytes,
    verifying_key: VerifyingKey,
}

impl SigningKey {
    pub fn generate() -> Result<SigningKey, KeyGenerationError> {
        let (private_key, public_key) = OpenMlsRustCrypto::default()
            .crypto()
            .signature_key_gen(DEFAULT_SIGNATURE_SCHEME)
            .map_err(|_| KeyGenerationError::KeypairGeneration)?;
        Ok(Self {
            signing_key: SecretBytes::from(private_key),
            verifying_key: VerifyingKey(public_key),
        })
    }

    pub fn verifying_key(&self) -> &VerifyingKey {
        &self.verifying_key
    }

    pub(super) fn expose_secret(&self) -> &SecretBytes {
        &self.signing_key
    }
}

#[cfg(feature = "sqlite")]
mod sqlite {
    use rusqlite::{types::FromSql, ToSql};

    use crate::codec::PhnxCodec;

    use super::{SigningKey, VerifyingKey};

    impl FromSql for VerifyingKey {
        fn column_result(
            value: rusqlite::types::ValueRef<'_>,
        ) -> rusqlite::types::FromSqlResult<Self> {
            let bytes = value.as_blob()?;
            Ok(VerifyingKey(bytes.to_vec()))
        }
    }

    impl ToSql for VerifyingKey {
        fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
            Ok(rusqlite::types::ToSqlOutput::Borrowed(
                rusqlite::types::ValueRef::Blob(&self.0),
            ))
        }
    }

    impl FromSql for SigningKey {
        fn column_result(
            value: rusqlite::types::ValueRef<'_>,
        ) -> rusqlite::types::FromSqlResult<Self> {
            let bytes = value.as_blob()?;
            let signing_key = PhnxCodec::from_slice(bytes)
                .map_err(|e| rusqlite::types::FromSqlError::Other(Box::new(e)))?;
            Ok(signing_key)
        }
    }

    impl ToSql for SigningKey {
        fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
            let bytes = PhnxCodec::to_vec(self)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            Ok(rusqlite::types::ToSqlOutput::Owned(
                rusqlite::types::Value::Blob(bytes),
            ))
        }
    }
}
