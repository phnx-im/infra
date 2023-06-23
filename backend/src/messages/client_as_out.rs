// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::openmls::prelude::GroupId;
use tls_codec::{DeserializeBytes, Serialize, TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::{
    auth_service::{
        credentials::{
            keys::AsIntermediateVerifyingKey, AsCredential, AsIntermediateCredential,
            ClientCredential, CredentialFingerprint, ExpirationData,
            VerifiableAsIntermediateCredential, VerifiableClientCredential,
        },
        errors::AsVerificationError,
        storage_provider_trait::{AsEphemeralStorageProvider, AsStorageProvider},
        AsClientId, OpaqueLoginResponse, OpaqueRegistrationRecord, OpaqueRegistrationResponse,
        UserName,
    },
    crypto::{
        ear::{
            keys::{ClientCredentialEarKey, GroupStateEarKey, SignatureEarKey},
            GenericDeserializable,
        },
        hpke::HpkeDecryptable,
        kdf::keys::RatchetSecret,
        signatures::{
            signable::{Signature, Verifiable, VerifiedStruct},
            traits::SignatureVerificationError,
        },
        ConnectionDecryptionKey, ConnectionEncryptionKey, RatchetEncryptionKey,
    },
};

use super::{
    client_as::{
        AsAuthMethod, AsClientConnectionPackageParams, AsCredentialsParams,
        AsDequeueMessagesParams, AsDequeueMessagesResponse, AsPublishConnectionPackagesParams,
        ClientCredentialAuthenticator, ConnectionEstablishmentPackageTbs, ConnectionPackage,
        ConnectionPackageTbs, DeleteClientParams, DeleteUserParams,
        EncryptedConnectionEstablishmentPackage, EnqueueMessageParams, FinishClientAdditionParams,
        FriendshipPackage, Init2FactorAuthResponse, InitUserRegistrationParams,
        Initiate2FaAuthenticationParams, InitiateClientAdditionParams, IssueTokensParams,
        IssueTokensResponse, NoAuth, TwoFactorAuthenticator, UserClientsParams,
        UserConnectionPackagesParams, VerifiedAsRequestParams,
    },
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
    DequeueMessages(AsDequeueMessagesResponse),
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
    pub user_name: UserName,
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

    pub(crate) fn auth_method(self) -> AsAuthMethod {
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

/// Wrapper struct around a message from a client to the AS. It does not
/// implement the [`Verifiable`] trait, but instead is verified depending on the
/// verification method of the individual payload.
#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub struct VerifiableClientToAsMessage {
    message: ClientToAsMessageIn,
}

impl VerifiableClientToAsMessage {
    /// Verify/authenticate the message. The authentication method depends on
    /// the request type and is specified for each request in `auth_method`.
    pub(crate) async fn verify<Asp: AsStorageProvider, Eph: AsEphemeralStorageProvider>(
        self,
        as_storage_provider: &Asp,
        ephemeral_storage_provider: &Eph,
    ) -> Result<VerifiedAsRequestParams, AsVerificationError> {
        let parameters = match self.message.auth_method() {
            // No authentication at all. We just return the parameters without
            // verification.
            AsAuthMethod::None(params) => params,
            // Authentication via client credential. We load the client
            // credential from the client's record and use it to verify the
            // request.
            AsAuthMethod::ClientCredential(cca) => {
                // Depending on the request type, we either load the client
                // credential from the persistend storage, or the ephemeral
                // storage.
                if matches!(
                    *cca.payload,
                    VerifiedAsRequestParams::FinishUserRegistration(_)
                ) {
                    tracing::info!("Loading client credential from ephemeral storage.");
                    let client_credential = ephemeral_storage_provider
                        .load_credential(&cca.client_id)
                        .await
                        .ok_or(AsVerificationError::UnknownClient)?;
                    cca.verify(client_credential.verifying_key())
                        .map_err(|_| AsVerificationError::AuthenticationFailed)?
                } else {
                    tracing::info!("Loading client credential from persistent storage.");
                    let client_record = as_storage_provider
                        .load_client(&cca.client_id)
                        .await
                        .ok_or(AsVerificationError::UnknownClient)?;
                    cca.verify(client_record.credential.verifying_key())
                        .map_err(|_| AsVerificationError::AuthenticationFailed)?
                }
            }
            // 2-Factor authentication using a signature by the client
            // credential, as well as an OPAQUE login flow. This requires that
            // the client has first called the endpoint to initiate the OPAQUE
            // login flow.
            // We load the pending OPAQUE login state from the ephemeral
            // database and complete the OPAQUE flow. If that is successful, we
            // verify the signature (which spans the OPAQUE data sent by the
            // client).
            // After successful verification, we delete the entry from the
            // ephemeral DB.
            // TODO: We currently store the credential of the client to be added
            // along with the OPAQUE entry. This is not great, since we can't
            // really return it from here. For now, we just load it again from
            // the processing function.
            AsAuthMethod::Client2Fa(auth_info) => {
                tracing::info!("Authenticating 2FA request");
                // We authenticate opaque first.
                let client_id = &auth_info.client_credential_auth.client_id.clone();
                let (_client_credential, opaque_state) = ephemeral_storage_provider
                    .load_client_login_state(client_id)
                    .await
                    .map_err(|_| AsVerificationError::StorageError)?
                    .ok_or(AsVerificationError::UnknownClient)?;
                // Finish the OPAQUE handshake
                opaque_state
                    .finish(auth_info.opaque_finish.client_message)
                    .map_err(|e| {
                        tracing::error!("Error during OPAQUE login handshake: {e}");
                        AsVerificationError::AuthenticationFailed
                    })?;

                let client_record = as_storage_provider
                    .load_client(client_id)
                    .await
                    .ok_or(AsVerificationError::UnknownClient)?;
                let verified_params = auth_info
                    .client_credential_auth
                    .verify(client_record.credential.verifying_key())
                    .map_err(|_| AsVerificationError::AuthenticationFailed)?;
                ephemeral_storage_provider
                    .delete_client_login_state(client_id)
                    .await
                    .map_err(|_| AsVerificationError::StorageError)?;
                verified_params
            }
            // Authentication using only the user's password via an OPAQUE login flow.
            AsAuthMethod::User(user_auth) => {
                let opaque_state = ephemeral_storage_provider
                    .load_user_login_state(&user_auth.user_name)
                    .await
                    .map_err(|_| AsVerificationError::StorageError)?
                    .ok_or(AsVerificationError::UnknownUser)?;
                // Finish the OPAQUE handshake
                opaque_state
                    .finish(user_auth.opaque_finish.client_message)
                    .map_err(|e| {
                        tracing::error!("Error during OPAQUE login handshake: {e}");
                        AsVerificationError::AuthenticationFailed
                    })?;

                ephemeral_storage_provider
                    .delete_user_login_state(&user_auth.user_name)
                    .await
                    .map_err(|_| AsVerificationError::StorageError)?;
                *user_auth.payload
            }
        };
        Ok(parameters)
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
    connection_group_signature_ear_key: SignatureEarKey,
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

    pub fn verify_all(
        self,
        as_intermediate_credentials: &[AsIntermediateCredential],
    ) -> ConnectionEstablishmentPackageTbs {
        let as_credential = as_intermediate_credentials
            .iter()
            .find(|as_cred| {
                &as_cred.fingerprint().unwrap()
                    == self.payload.sender_client_credential.signer_fingerprint()
            })
            .unwrap();
        let sender_client_credential: ClientCredential = self
            .payload
            .sender_client_credential
            .verify(as_credential.verifying_key())
            .unwrap();
        ConnectionEstablishmentPackageTbs {
            sender_client_credential,
            connection_group_id: self.payload.connection_group_id,
            connection_group_ear_key: self.payload.connection_group_ear_key,
            connection_group_credential_key: self.payload.connection_group_credential_key,
            connection_group_signature_ear_key: self.payload.connection_group_signature_ear_key,
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
