// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use tls_codec::{Serialize as _, TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::{
    credentials::{
        AsCredential, ClientCredential, ClientCredentialPayload, CredentialFingerprint,
        VerifiableAsIntermediateCredential, VerifiableClientCredential,
        keys::{AsIntermediateVerifyingKey, ClientSignature},
    },
    crypto::{
        ConnectionEncryptionKey,
        indexed_aead::{
            ciphertexts::IndexedCiphertext,
            keys::{UserProfileKeyIndex, UserProfileKeyType},
        },
        signatures::{private_keys::SignatureVerificationError, signable::Verifiable},
    },
    identifiers::UserId,
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

#[derive(Debug)]
pub struct AsCredentialsResponseIn {
    // TODO: We might want a Verifiable... type variant here that ensures that
    // this is matched against the local trust store or something.
    pub as_credentials: Vec<AsCredential>,
    pub as_intermediate_credentials: Vec<VerifiableAsIntermediateCredential>,
    pub revoked_credentials: Vec<CredentialFingerprint>,
}

#[derive(Debug)]
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
    pub(super) signature: ClientSignature,
}

impl ConnectionPackageIn {
    pub fn new(payload: ConnectionPackageTbsIn, signature: ClientSignature) -> Self {
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
    pub(super) signature: ClientSignature,
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

#[derive(Debug)]
pub struct RegisterUserParamsIn {
    pub client_payload: ClientCredentialPayload,
    pub encrypted_user_profile: EncryptedUserProfile,
}

pub struct GetUserProfileParams {
    pub user_id: UserId,
    pub key_index: UserProfileKeyIndex,
}

#[derive(Debug)]
pub struct EncryptedUserProfileCtype;
pub type EncryptedUserProfile = IndexedCiphertext<UserProfileKeyType, EncryptedUserProfileCtype>;

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct GetUserProfileResponse {
    pub encrypted_user_profile: EncryptedUserProfile,
}

#[derive(Debug)]
pub struct UpdateUserProfileParamsTbs {
    pub user_id: UserId,
    pub user_profile: EncryptedUserProfile,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct StageUserProfileParamsTbs {
    pub user_id: UserId,
    pub user_profile: EncryptedUserProfile,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct StageUserProfileParams {
    payload: StageUserProfileParamsTbs,
    signature: ClientSignature,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct MergeUserProfileParamsTbs {
    pub user_id: UserId,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct MergeUserProfileParams {
    payload: MergeUserProfileParamsTbs,
    signature: ClientSignature,
}
