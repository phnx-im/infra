// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{openmls::prelude::SignaturePublicKey, openmls_rust_crypto::OpenMlsRustCrypto};
use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::crypto::errors::KeyGenerationError;

use super::{
    private_keys::{SigningKey, VerifyingKey},
    traits::{SigningKeyBehaviour, VerifyingKeyBehaviour},
};

#[derive(Debug)]
pub struct LeafVerifyingKey(VerifyingKey);

impl VerifyingKeyBehaviour for LeafVerifyingKey {}

impl AsRef<VerifyingKey> for LeafVerifyingKey {
    fn as_ref(&self) -> &VerifyingKey {
        &self.0
    }
}

impl From<&SignaturePublicKey> for LeafVerifyingKey {
    fn from(pk_ref: &SignaturePublicKey) -> Self {
        Self(pk_ref.clone().into())
    }
}

#[derive(
    Clone, PartialEq, Serialize, Deserialize, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize,
)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
pub struct QsClientVerifyingKey(VerifyingKey);

impl QsClientVerifyingKey {
    #[cfg(any(test, feature = "test_utils"))]
    pub fn new_for_test(verifying_key: VerifyingKey) -> Self {
        Self(verifying_key)
    }
}

impl AsRef<VerifyingKey> for QsClientVerifyingKey {
    fn as_ref(&self) -> &VerifyingKey {
        &self.0
    }
}

impl VerifyingKeyBehaviour for QsClientVerifyingKey {}

#[derive(Clone, Serialize, Deserialize)]
pub struct QsClientSigningKey(SigningKey);

impl QsClientSigningKey {
    pub fn random() -> Result<Self, KeyGenerationError> {
        let rust_crypto = OpenMlsRustCrypto::default();
        let signing_key = SigningKey::generate()?;
        Ok(Self(signing_key))
    }

    pub fn verifying_key(&self) -> QsClientVerifyingKey {
        QsClientVerifyingKey(self.0.verifying_key().clone())
    }
}

impl AsRef<SigningKey> for QsClientSigningKey {
    fn as_ref(&self) -> &SigningKey {
        &self.0
    }
}

impl super::traits::SigningKeyBehaviour for QsClientSigningKey {}

#[derive(
    Clone, PartialEq, Serialize, Deserialize, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize,
)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
pub struct QsUserVerifyingKey(VerifyingKey);

impl QsUserVerifyingKey {
    #[cfg(any(test, feature = "test_utils"))]
    pub fn new_for_test(verifying_key: VerifyingKey) -> Self {
        Self(verifying_key)
    }
}

impl AsRef<VerifyingKey> for QsUserVerifyingKey {
    fn as_ref(&self) -> &VerifyingKey {
        &self.0
    }
}

impl VerifyingKeyBehaviour for QsUserVerifyingKey {}

#[derive(Clone, Serialize, Deserialize)]
pub struct QsUserSigningKey(SigningKey);

impl QsUserSigningKey {
    pub fn generate() -> Result<Self, KeyGenerationError> {
        let signing_key = SigningKey::generate()?;
        Ok(Self(signing_key))
    }

    pub fn verifying_key(&self) -> QsUserVerifyingKey {
        QsUserVerifyingKey(self.0.verifying_key().clone())
    }
}

impl AsRef<SigningKey> for QsUserSigningKey {
    fn as_ref(&self) -> &SigningKey {
        &self.0
    }
}

impl SigningKeyBehaviour for QsUserSigningKey {}
