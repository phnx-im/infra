// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use tls_codec::{Serialize as _, TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::{
    credentials::{
        AsCredential, ClientCredential, ClientCredentialPayload, CredentialFingerprint,
        VerifiableAsIntermediateCredential, VerifiableClientCredential,
        keys::AsIntermediateVerifyingKey,
    },
    crypto::{
        ConnectionEncryptionKey, RatchetEncryptionKey,
        ear::Ciphertext,
        kdf::keys::RatchetSecret,
        signatures::{
            private_keys::SignatureVerificationError,
            signable::{Signature, Verifiable},
        },
    },
    identifiers::AsClientId,
    time::ExpirationData,
};

use super::{
    MlsInfraVersion,
    client_as::{ConnectionPackage, ConnectionPackageTbs},
};

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub struct UserConnectionPackagesResponseIn {
    pub connection_packages: Vec<ConnectionPackageIn>,
}

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub struct UserClientsResponseIn {
    pub client_credentials: Vec<VerifiableClientCredential>,
}

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub struct AsCredentialsResponseIn {
    // TODO: We might want a Verifiable... type variant here that ensures that
    // this is matched against the local trust store or something.
    pub as_credentials: Vec<AsCredential>,
    pub as_intermediate_credentials: Vec<VerifiableAsIntermediateCredential>,
    pub revoked_credentials: Vec<CredentialFingerprint>,
}

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub struct RegisterUserResponseIn {
    pub client_credential: VerifiableClientCredential,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct ConnectionPackageTbsIn {
    pub protocol_version: MlsInfraVersion,
    pub encryption_key: ConnectionEncryptionKey,
    pub lifetime: ExpirationData,
    pub client_credential: VerifiableClientCredential,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct ConnectionPackageIn {
    pub(super) payload: ConnectionPackageTbsIn,
    pub(super) signature: Signature,
}

impl ConnectionPackageIn {
    pub fn new(payload: ConnectionPackageTbsIn, signature: Signature) -> Self {
        Self { payload, signature }
    }

    pub fn client_credential_signer_fingerprint(&self) -> &CredentialFingerprint {
        self.payload.client_credential.signer_fingerprint()
    }

    pub fn verify(
        self,
        credential_verifying_key: &AsIntermediateVerifyingKey,
    ) -> Result<ConnectionPackage, SignatureVerificationError> {
        let client_credential: ClientCredential = self
            .payload
            .client_credential
            .verify(credential_verifying_key)?;
        let verifying_key = client_credential.verifying_key().clone();
        let verifiable_connection_package = VerifiableConnectionPackage {
            payload: ConnectionPackageTbs {
                protocol_version: self.payload.protocol_version,
                encryption_key: self.payload.encryption_key,
                lifetime: self.payload.lifetime,
                client_credential,
            },
            signature: self.signature,
        };
        verifiable_connection_package.verify(&verifying_key)
    }
}

#[derive(Debug)]
pub struct VerifiableConnectionPackage {
    pub(super) payload: ConnectionPackageTbs,
    pub(super) signature: Signature,
}

impl Verifiable for VerifiableConnectionPackage {
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

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct RegisterUserParamsIn {
    pub client_payload: ClientCredentialPayload,
    pub queue_encryption_key: RatchetEncryptionKey,
    pub initial_ratchet_secret: RatchetSecret,
    pub encrypted_user_profile: EncryptedUserProfile,
}

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub struct AsPublishConnectionPackagesParamsIn {
    payload: AsPublishConnectionPackagesParamsTbsIn,
    signature: Signature,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct AsPublishConnectionPackagesParamsTbsIn {
    pub client_id: AsClientId,
    pub connection_packages: Vec<ConnectionPackageIn>,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct GetUserProfileParams {
    pub client_id: AsClientId,
}

#[derive(Debug)]
pub struct EncryptedUserProfileCtype;
pub type EncryptedUserProfile = Ciphertext<EncryptedUserProfileCtype>;

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct GetUserProfileResponse {
    pub encrypted_user_profile: EncryptedUserProfile,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct UpdateUserProfileParamsTbs {
    pub client_id: AsClientId,
    pub user_profile: EncryptedUserProfile,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct UpdateUserProfileParams {
    payload: UpdateUserProfileParamsTbs,
    signature: Signature,
}
