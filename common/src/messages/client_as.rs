// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::openmls_traits::types::HpkeCiphertext;

use tls_codec::{Serialize as TlsSerializeTrait, TlsDeserializeBytes, TlsSerialize, TlsSize};

use serde::{Deserialize, Serialize};

use crate::{
    credentials::{
        AsCredential, AsIntermediateCredential, ClientCredential, ClientCredentialPayload,
        CredentialFingerprint,
        keys::{ClientKeyType, ClientSignature},
    },
    crypto::{
        ConnectionEncryptionKey, RatchetEncryptionKey,
        ear::Ciphertext,
        kdf::keys::RatchetSecret,
        signatures::signable::{Signable, SignedStruct, VerifiedStruct},
    },
    time::ExpirationData,
};

use super::{
    MlsInfraVersion,
    client_as_out::{EncryptedUserProfile, VerifiableConnectionPackage},
};

mod private_mod {
    #[derive(Default)]
    pub struct Seal;
}

#[derive(Debug, Clone, PartialEq, Eq, TlsSerialize, TlsSize, Serialize, Deserialize)]
pub struct ConnectionPackageTbs {
    pub protocol_version: MlsInfraVersion,
    pub encryption_key: ConnectionEncryptionKey,
    pub lifetime: ExpirationData,
    pub client_credential: ClientCredential,
}

impl ConnectionPackageTbs {
    pub fn new(
        protocol_version: MlsInfraVersion,
        encryption_key: ConnectionEncryptionKey,
        lifetime: ExpirationData,
        client_credential: ClientCredential,
    ) -> Self {
        Self {
            protocol_version,
            encryption_key,
            lifetime,
            client_credential,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, TlsSerialize, TlsSize, Serialize, Deserialize)]
pub struct ConnectionPackage {
    payload: ConnectionPackageTbs,
    signature: ClientSignature,
}

impl ConnectionPackage {
    pub fn new(payload: ConnectionPackageTbs, signature: ClientSignature) -> Self {
        Self { payload, signature }
    }

    pub fn into_parts(self) -> (ConnectionPackageTbs, ClientSignature) {
        (self.payload, self.signature)
    }

    pub fn client_credential(&self) -> &ClientCredential {
        &self.payload.client_credential
    }

    pub fn encryption_key(&self) -> &ConnectionEncryptionKey {
        &self.payload.encryption_key
    }

    pub fn client_credential_signer_fingerprint(&self) -> &CredentialFingerprint {
        self.payload.client_credential.signer_fingerprint()
    }

    #[cfg(feature = "test_utils")]
    pub fn new_for_test(payload: ConnectionPackageTbs, signature: ClientSignature) -> Self {
        Self { payload, signature }
    }
}

impl VerifiedStruct<VerifiableConnectionPackage> for ConnectionPackage {
    type SealingType = private_mod::Seal;

    fn from_verifiable(verifiable: VerifiableConnectionPackage, _seal: Self::SealingType) -> Self {
        Self {
            payload: verifiable.payload,
            signature: verifiable.signature,
        }
    }
}

impl Signable for ConnectionPackageTbs {
    type SignedOutput = ConnectionPackage;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        "ConnectionPackage"
    }
}

impl SignedStruct<ConnectionPackageTbs, ClientKeyType> for ConnectionPackage {
    fn from_payload(payload: ConnectionPackageTbs, signature: ClientSignature) -> Self {
        Self { payload, signature }
    }
}

// === User ===

#[derive(Debug)]
pub struct RegisterUserParams {
    pub client_payload: ClientCredentialPayload,
    pub queue_encryption_key: RatchetEncryptionKey,
    pub initial_ratchet_secret: RatchetSecret,
    pub encrypted_user_profile: EncryptedUserProfile,
}

#[derive(Debug)]
pub struct RegisterUserResponse {
    pub client_credential: ClientCredential,
}

// === Client ===

#[derive(Debug)]
pub struct EncryptedFriendshipPackageCtype;
pub type EncryptedFriendshipPackage = Ciphertext<EncryptedFriendshipPackageCtype>;

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub struct EncryptedConnectionOffer {
    ciphertext: HpkeCiphertext,
}

impl EncryptedConnectionOffer {
    pub fn into_ciphertext(self) -> HpkeCiphertext {
        self.ciphertext
    }
}

impl AsRef<HpkeCiphertext> for EncryptedConnectionOffer {
    fn as_ref(&self) -> &HpkeCiphertext {
        &self.ciphertext
    }
}

impl From<HpkeCiphertext> for EncryptedConnectionOffer {
    fn from(ciphertext: HpkeCiphertext) -> Self {
        Self { ciphertext }
    }
}

// === Anonymous requests ===

#[derive(Debug)]
pub struct AsCredentialsParams {}

#[derive(Debug)]
pub struct AsCredentialsResponse {
    pub as_credentials: Vec<AsCredential>,
    pub as_intermediate_credentials: Vec<AsIntermediateCredential>,
    pub revoked_credentials: Vec<CredentialFingerprint>,
}
