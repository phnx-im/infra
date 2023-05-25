// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls_basic_credential::SignatureKeyPair;

use super::*;

pub(crate) const CIPHERSUITE: Ciphersuite =
    Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;

pub struct SelfUser {
    pub(crate) crypto_backend: OpenMlsRustCrypto,
    pub(crate) username: String,
    pub(crate) credential_with_key: CredentialWithKey,
    pub(crate) signer: SignatureKeyPair,
}

impl SelfUser {
    /// Create a new user with the given name and a fresh set of credentials.
    pub fn new(username: String) -> Self {
        let crypto_backend = OpenMlsRustCrypto::default();
        let credential =
            Credential::new(username.as_bytes().to_vec(), CredentialType::Basic).unwrap();
        let signer = SignatureKeyPair::new(SignatureScheme::from(CIPHERSUITE)).unwrap();
        signer.store(crypto_backend.key_store()).unwrap();

        Self {
            crypto_backend,
            username,
            credential_with_key: CredentialWithKey {
                credential,
                signature_key: signer.public().to_vec().into(),
            },
            signer,
        }
    }

    pub(crate) fn generate_keypackage(&self) -> KeyPackage {
        KeyPackage::builder()
            .build(
                CryptoConfig {
                    ciphersuite: CIPHERSUITE,
                    version: ProtocolVersion::Mls10,
                },
                &self.crypto_backend,
                &self.signer,
                self.credential_with_key.clone(),
            )
            .unwrap()
    }

    pub fn signer(&self) -> &SignatureKeyPair {
        &self.signer
    }
}