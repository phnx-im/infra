// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::openmls::prelude::GroupId;
use tls_codec::{DeserializeBytes, Serialize, TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::{
    credentials::{
        keys::AsIntermediateVerifyingKey, AsCredential, ClientCredential, CredentialFingerprint,
        VerifiableAsIntermediateCredential, VerifiableClientCredential,
    },
    crypto::{
        ear::{
            keys::{
                ClientCredentialEarKey, FriendshipPackageEarKey, GroupStateEarKey,
                SignatureEarKeyWrapperKey,
            },
            GenericDeserializable,
        },
        hpke::HpkeDecryptable,
        kdf::keys::RatchetSecret,
        opaque::{OpaqueLoginResponse, OpaqueRegistrationRecord, OpaqueRegistrationResponse},
        signatures::{
            signable::{Signature, Verifiable, VerifiedStruct},
            traits::SignatureVerificationError,
        },
        ConnectionDecryptionKey, ConnectionEncryptionKey, RatchetEncryptionKey,
    },
    identifiers::AsClientId,
    time::ExpirationData,
};

use super::{
    client_as::{
        AsAuthMethod, AsClientConnectionPackageParams, AsCredentialsParams,
        AsDequeueMessagesParams, AsPublishConnectionPackagesParams, ClientCredentialAuthenticator,
        ConnectionEstablishmentPackageTbs, ConnectionPackage, ConnectionPackageTbs,
        DeleteClientParams, DeleteUserParams, EncryptedConnectionEstablishmentPackage,
        EnqueueMessageParams, FinishClientAdditionParams, FriendshipPackage,
        Init2FactorAuthResponse, InitUserRegistrationParams, Initiate2FaAuthenticationParams,
        InitiateClientAdditionParams, IssueTokensParams, IssueTokensResponse, NoAuth,
        TwoFactorAuthenticator, UserClientsParams, UserConnectionPackagesParams,
        VerifiedAsRequestParams,
    },
    client_qs::DequeueMessagesResponse,
    MlsInfraVersion,
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
    pub opaque_login_response: OpaqueLoginResponse,
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
    pub opaque_registration_response: OpaqueRegistrationResponse,
}

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub enum AsProcessResponseIn {
    Ok,
    Init2FactorAuth(Init2FactorAuthResponse),
    DequeueMessages(DequeueMessagesResponse),
    ClientConnectionPackage(AsClientConnectionPackageResponseIn),
    IssueTokens(IssueTokensResponse),
    UserConnectionPackages(UserConnectionPackagesResponseIn),
    InitiateClientAddition(InitClientAdditionResponseIn),
    UserClients(UserClientsResponseIn),
    AsCredentials(AsCredentialsResponseIn),
    InitUserRegistration(InitUserRegistrationResponseIn),
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

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn label(&self) -> &str {
        "ConnectionPackage"
    }
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct FinishUserRegistrationParamsTbsIn {
    pub client_id: AsClientId,
    pub queue_encryption_key: RatchetEncryptionKey,
    pub initial_ratchet_secret: RatchetSecret,
    pub connection_packages: Vec<ConnectionPackageIn>,
    pub opaque_registration_record: OpaqueRegistrationRecord,
}

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub struct FinishUserRegistrationParamsIn {
    payload: FinishUserRegistrationParamsTbsIn,
    signature: Signature,
}

impl ClientCredentialAuthenticator for FinishUserRegistrationParamsIn {
    type Tbs = FinishUserRegistrationParamsTbsIn;

    fn client_id(&self) -> AsClientId {
        self.payload.client_id.clone()
    }

    fn into_payload(self) -> VerifiedAsRequestParams {
        VerifiedAsRequestParams::FinishUserRegistration(self.payload)
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    const LABEL: &'static str = "Finish User Registration Parameters";
}

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub struct ClientToAsMessageIn {
    _version: MlsInfraVersion,
    // This essentially includes the wire format.
    body: AsRequestParamsIn,
}

impl ClientToAsMessageIn {
    pub fn new(body: AsRequestParamsIn) -> Self {
        Self {
            _version: MlsInfraVersion::default(),
            body,
        }
    }

    pub fn auth_method(self) -> AsAuthMethod {
        self.body.auth_method()
    }
}

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub enum AsRequestParamsIn {
    Initiate2FaAuthentication(Initiate2FaAuthenticationParams),
    InitUserRegistration(InitUserRegistrationParams),
    FinishUserRegistration(FinishUserRegistrationParamsIn),
    DeleteUser(DeleteUserParams),
    InitiateClientAddition(InitiateClientAdditionParams),
    FinishClientAddition(FinishClientAdditionParams),
    DeleteClient(DeleteClientParams),
    DequeueMessages(AsDequeueMessagesParams),
    PublishConnectionPackages(AsPublishConnectionPackagesParams),
    ClientConnectionPackage(AsClientConnectionPackageParams),
    UserClients(UserClientsParams),
    UserConnectionPackages(UserConnectionPackagesParams),
    EnqueueMessage(EnqueueMessageParams),
    AsCredentials(AsCredentialsParams),
    IssueTokens(IssueTokensParams),
}

impl AsRequestParamsIn {
    pub(crate) fn auth_method(self) -> AsAuthMethod {
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
            Self::Initiate2FaAuthentication(params) => {
                AsAuthMethod::ClientCredential(params.credential_auth_info())
            }
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
            // We verify user registration finish requests like a
            // ClientCredentialAuth request and then additionally complete the
            // OPAQUE registration afterwards.
            Self::FinishUserRegistration(params) => {
                AsAuthMethod::ClientCredential(params.credential_auth_info())
            }
            // Requests not requiring any authentication
            Self::UserClients(params) => AsAuthMethod::None(params.into_verified()),
            Self::UserConnectionPackages(params) => AsAuthMethod::None(params.into_verified()),
            Self::EnqueueMessage(params) => AsAuthMethod::None(params.into_verified()),
            Self::InitUserRegistration(params) => AsAuthMethod::None(params.into_verified()),
            Self::InitiateClientAddition(params) => AsAuthMethod::None(params.into_verified()),
            Self::AsCredentials(params) => AsAuthMethod::None(params.into_verified()),
        }
    }
}

mod private_mod {
    #[derive(Default)]
    pub struct Seal;
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Clone)]
pub struct ConnectionEstablishmentPackageTbsIn {
    sender_client_credential: VerifiableClientCredential,
    connection_group_id: GroupId,
    connection_group_ear_key: GroupStateEarKey,
    connection_group_credential_key: ClientCredentialEarKey,
    connection_group_signature_ear_key_wrapper_key: SignatureEarKeyWrapperKey,
    pub friendship_package_ear_key: FriendshipPackageEarKey,
    friendship_package: FriendshipPackage,
}

impl VerifiedStruct<ConnectionEstablishmentPackageIn> for ConnectionEstablishmentPackageTbsIn {
    type SealingType = private_mod::Seal;

    fn from_verifiable(
        verifiable: ConnectionEstablishmentPackageIn,
        _seal: Self::SealingType,
    ) -> Self {
        verifiable.payload
    }
}

#[derive(Debug, TlsDeserializeBytes, TlsSize, Clone)]
pub struct ConnectionEstablishmentPackageIn {
    payload: ConnectionEstablishmentPackageTbsIn,
    // TBS: All information above signed by the ClientCredential.
    signature: Signature,
}

impl GenericDeserializable for ConnectionEstablishmentPackageIn {
    type Error = tls_codec::Error;

    fn deserialize(bytes: &[u8]) -> Result<Self, Self::Error> {
        Self::tls_deserialize_exact(bytes)
    }
}

impl ConnectionEstablishmentPackageIn {
    pub fn sender_credential(&self) -> &VerifiableClientCredential {
        &self.payload.sender_client_credential
    }

    pub fn verify(
        self,
        verifying_key: &AsIntermediateVerifyingKey,
    ) -> ConnectionEstablishmentPackageTbs {
        let sender_client_credential: ClientCredential = self
            .payload
            .sender_client_credential
            .verify(verifying_key)
            .unwrap();
        ConnectionEstablishmentPackageTbs {
            sender_client_credential,
            connection_group_id: self.payload.connection_group_id,
            connection_group_ear_key: self.payload.connection_group_ear_key,
            connection_group_credential_key: self.payload.connection_group_credential_key,
            connection_group_signature_ear_key_wrapper_key: self
                .payload
                .connection_group_signature_ear_key_wrapper_key,
            friendship_package_ear_key: self.payload.friendship_package_ear_key,
            friendship_package: self.payload.friendship_package,
        }
    }
}

impl HpkeDecryptable<ConnectionDecryptionKey, EncryptedConnectionEstablishmentPackage>
    for ConnectionEstablishmentPackageIn
{
}

impl Verifiable for ConnectionEstablishmentPackageIn {
    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.payload.tls_serialize_detached()
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn label(&self) -> &str {
        "ConnectionEstablishmentPackageTBS"
    }
}
