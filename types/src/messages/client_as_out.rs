// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use tls_codec::{Serialize as _, TlsDeserializeBytes, TlsSerialize, TlsSize, TlsVarInt};

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
            signable::{Signable, Signature, SignedStruct, Verifiable},
        },
    },
    errors::version::VersionError,
    identifiers::AsClientId,
    time::ExpirationData,
};

use super::{
    ApiVersion, MlsInfraVersion,
    client_as::{
        AsAuthMethod, AsClientConnectionPackageParams, AsCredentialsParams,
        AsDequeueMessagesParams, ClientCredentialAuthenticator, ConnectionPackage,
        ConnectionPackageTbs, DeleteClientParams, DeleteUserParams, EnqueueMessageParams,
        FinishClientAdditionParams, InitiateClientAdditionParams, IssueTokensParams,
        IssueTokensResponse, NoAuth, SUPPORTED_AS_API_VERSIONS, TwoFactorAuthenticator,
        UserClientsParams, UserConnectionPackagesParams, VerifiedAsRequestParams,
    },
    client_qs::DequeueMessagesResponse,
};

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub struct AsClientConnectionPackageResponseIn {
    pub connection_package: Option<ConnectionPackageIn>,
}

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub struct UserConnectionPackagesResponseIn {
    pub connection_packages: Vec<ConnectionPackageIn>,
}

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub struct InitClientAdditionResponseIn {
    pub client_credential: VerifiableClientCredential,
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
pub struct InitUserRegistrationResponseIn {
    pub client_credential: VerifiableClientCredential,
}

#[expect(clippy::large_enum_variant)]
pub enum AsVersionedProcessResponseIn {
    Other(ApiVersion),
    Alpha(AsProcessResponseIn),
}

impl AsVersionedProcessResponseIn {
    fn version(&self) -> ApiVersion {
        match self {
            Self::Other(version) => *version,
            Self::Alpha(_) => ApiVersion::new(1).expect("infallible"),
        }
    }

    pub fn into_unversioned(self) -> Result<AsProcessResponseIn, VersionError> {
        match self {
            Self::Alpha(response) => Ok(response),
            Self::Other(version) => Err(VersionError::new(version, SUPPORTED_AS_API_VERSIONS)),
        }
    }
}

impl tls_codec::Size for AsVersionedProcessResponseIn {
    fn tls_serialized_len(&self) -> usize {
        match self {
            AsVersionedProcessResponseIn::Other(_) => {
                self.version().tls_value().tls_serialized_len()
            }
            AsVersionedProcessResponseIn::Alpha(response) => {
                self.version().tls_value().tls_serialized_len() + response.tls_serialized_len()
            }
        }
    }
}

impl tls_codec::DeserializeBytes for AsVersionedProcessResponseIn {
    fn tls_deserialize_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error> {
        let (version, bytes) = TlsVarInt::tls_deserialize_bytes(bytes)?;
        match version.value() {
            1 => {
                let (response, bytes) = AsProcessResponseIn::tls_deserialize_bytes(bytes)?;
                Ok((Self::Alpha(response), bytes))
            }
            _ => Ok((Self::Other(ApiVersion::from_tls_value(version)), bytes)),
        }
    }
}

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub enum AsProcessResponseIn {
    Ok,
    DequeueMessages(DequeueMessagesResponse),
    ClientConnectionPackage(AsClientConnectionPackageResponseIn),
    IssueTokens(IssueTokensResponse),
    UserConnectionPackages(UserConnectionPackagesResponseIn),
    InitiateClientAddition(InitClientAdditionResponseIn),
    UserClients(UserClientsResponseIn),
    AsCredentials(AsCredentialsResponseIn),
    InitUserRegistration(InitUserRegistrationResponseIn),
    GetUserProfile(GetUserProfileResponse),
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
pub struct InitUserRegistrationParamsIn {
    pub client_payload: ClientCredentialPayload,
    pub queue_encryption_key: RatchetEncryptionKey,
    pub initial_ratchet_secret: RatchetSecret,
    pub encrypted_user_profile: EncryptedUserProfile,
}

impl NoAuth for InitUserRegistrationParamsIn {
    fn into_verified(self) -> VerifiedAsRequestParams {
        VerifiedAsRequestParams::InitUserRegistration(self)
    }
}

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub struct AsPublishConnectionPackagesParamsIn {
    payload: AsPublishConnectionPackagesParamsTbsIn,
    signature: Signature,
}

impl ClientCredentialAuthenticator for AsPublishConnectionPackagesParamsIn {
    type Tbs = AsClientId;

    fn client_id(&self) -> AsClientId {
        self.payload.client_id.clone()
    }

    fn into_payload(self) -> VerifiedAsRequestParams {
        VerifiedAsRequestParams::PublishConnectionPackages(self.payload)
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    const LABEL: &'static str = "Publish ConnectionPackages Parameters";
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

impl NoAuth for GetUserProfileParams {
    fn into_verified(self) -> VerifiedAsRequestParams {
        VerifiedAsRequestParams::GetUserProfile(self)
    }
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

impl ClientCredentialAuthenticator for UpdateUserProfileParams {
    type Tbs = UpdateUserProfileParamsTbs;

    fn client_id(&self) -> AsClientId {
        self.payload.client_id.clone()
    }

    fn into_payload(self) -> VerifiedAsRequestParams {
        VerifiedAsRequestParams::UpdateUserProfile(self.payload)
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    const LABEL: &'static str = "Finish User Registration Parameters";
}

impl Signable for UpdateUserProfileParamsTbs {
    type SignedOutput = UpdateUserProfileParams;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        UpdateUserProfileParams::LABEL
    }
}

impl SignedStruct<UpdateUserProfileParamsTbs> for UpdateUserProfileParams {
    fn from_payload(payload: UpdateUserProfileParamsTbs, signature: Signature) -> Self {
        Self { payload, signature }
    }
}

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub struct ClientToAsMessageIn {
    // This essentially includes the wire format.
    body: AsVersionedRequestParamsIn,
}

impl ClientToAsMessageIn {
    pub fn new(body: AsVersionedRequestParamsIn) -> Self {
        Self { body }
    }

    pub fn into_body(self) -> AsVersionedRequestParamsIn {
        self.body
    }
}

#[derive(Debug)]
#[expect(clippy::large_enum_variant)]
pub enum AsVersionedRequestParamsIn {
    Other(ApiVersion),
    Alpha(AsRequestParamsIn),
}

impl AsVersionedRequestParamsIn {
    pub fn version(&self) -> ApiVersion {
        match self {
            Self::Other(version) => *version,
            Self::Alpha(_) => ApiVersion::new(1).expect("infallible"),
        }
    }

    pub fn into_unversioned(self) -> Result<(AsRequestParamsIn, ApiVersion), VersionError> {
        let version = self.version();
        let params = match self {
            Self::Other(_) => {
                return Err(VersionError::new(version, SUPPORTED_AS_API_VERSIONS));
            }
            Self::Alpha(params) => params,
        };
        Ok((params, version))
    }
}

impl tls_codec::Size for AsVersionedRequestParamsIn {
    fn tls_serialized_len(&self) -> usize {
        match self {
            Self::Other(_) => self.version().tls_value().tls_serialized_len(),
            Self::Alpha(ds_request_params) => {
                self.version().tls_value().tls_serialized_len()
                    + ds_request_params.tls_serialized_len()
            }
        }
    }
}

impl tls_codec::DeserializeBytes for AsVersionedRequestParamsIn {
    fn tls_deserialize_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error> {
        let (version, bytes) = TlsVarInt::tls_deserialize_bytes(bytes)?;
        match version.value() {
            1 => {
                let (params, bytes) = AsRequestParamsIn::tls_deserialize_bytes(bytes)?;
                Ok((Self::Alpha(params), bytes))
            }
            _ => Ok((Self::Other(ApiVersion::from_tls_value(version)), bytes)),
        }
    }
}

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub enum AsRequestParamsIn {
    InitUserRegistration(InitUserRegistrationParamsIn),
    DeleteUser(DeleteUserParams),
    InitiateClientAddition(InitiateClientAdditionParams),
    FinishClientAddition(FinishClientAdditionParams),
    DeleteClient(DeleteClientParams),
    DequeueMessages(AsDequeueMessagesParams),
    PublishConnectionPackages(AsPublishConnectionPackagesParamsIn),
    ClientConnectionPackage(AsClientConnectionPackageParams),
    UserClients(UserClientsParams),
    UserConnectionPackages(UserConnectionPackagesParams),
    EnqueueMessage(EnqueueMessageParams),
    AsCredentials(AsCredentialsParams),
    IssueTokens(IssueTokensParams),
    GetUserProfile(GetUserProfileParams),
    UpdateUserProfile(UpdateUserProfileParams),
}

impl AsRequestParamsIn {
    pub fn into_auth_method(self) -> AsAuthMethod {
        match self {
            // Requests authenticated only by the user's password.
            // TODO: We should probably sign/verify the CSR with the verifying
            // key inside to prove ownership of the key.
            // TODO: For now, client addition is only verified by the user's
            // password, not with an additional client credential.
            Self::FinishClientAddition(params) => AsAuthMethod::User(params.user_auth()),
            // Requests authenticated using two factors
            Self::DeleteUser(params) => AsAuthMethod::Client2Fa(params.two_factor_auth_info()),
            // Requests signed by the client's client credential
            Self::DeleteClient(params) => {
                AsAuthMethod::ClientCredential(params.credential_auth_info())
            }
            Self::DequeueMessages(params) => {
                AsAuthMethod::ClientCredential(params.credential_auth_info())
            }
            Self::PublishConnectionPackages(params) => {
                AsAuthMethod::ClientCredential(params.credential_auth_info())
            }
            Self::ClientConnectionPackage(params) => {
                AsAuthMethod::ClientCredential(params.credential_auth_info())
            }
            Self::IssueTokens(params) => {
                AsAuthMethod::ClientCredential(params.credential_auth_info())
            }
            Self::UpdateUserProfile(params) => {
                AsAuthMethod::ClientCredential(params.credential_auth_info())
            }
            // Requests not requiring any authentication
            Self::UserClients(params) => AsAuthMethod::None(params.into_verified()),
            Self::UserConnectionPackages(params) => AsAuthMethod::None(params.into_verified()),
            Self::EnqueueMessage(params) => AsAuthMethod::None(params.into_verified()),
            Self::InitUserRegistration(params) => AsAuthMethod::None(params.into_verified()),
            Self::InitiateClientAddition(params) => AsAuthMethod::None(params.into_verified()),
            Self::AsCredentials(params) => AsAuthMethod::None(params.into_verified()),
            Self::GetUserProfile(params) => AsAuthMethod::None(params.into_verified()),
        }
    }
}
