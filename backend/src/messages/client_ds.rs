//! This module contains structs and enums that represent messages that are
//! passed between clients and the backend.
//! TODO: We should eventually factor this module out, together with the crypto
//! module, to allow re-use by the client implementation.

use mls_assist::{
    messages::{AssistedMessage, AssistedWelcome, SerializedAssistedMessage},
    GroupEpoch, GroupId, LeafNode, LeafNodeIndex, Sender, VerifiableGroupInfo,
};
use serde::{Deserialize, Serialize};
use tls_codec::{Deserialize as TlsDeserializeTrait, TlsDeserialize, TlsSerialize, TlsSize};
use utoipa::ToSchema;

use crate::{
    crypto::{
        ear::{
            keys::{GroupStateEarKey, RatchetKey},
            Ciphertext, EarEncryptable,
        },
        signatures::{
            keys::{LeafSignatureKey, UserAuthKey},
            signable::{Signature, Verifiable, VerifiedStruct},
        },
    },
    ds::group_state::{EncryptedCredentialChain, UserKeyHash},
    qs::{QsClientReference, VerifiableKeyPackageBatch},
};

use super::MlsInfraVersion;

mod private_mod {
    #[derive(Default)]
    pub(crate) struct Seal;
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, Clone, Serialize, Deserialize)]
pub struct DsFanoutPayload {
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct EncryptedDsMessage {
    payload: Ciphertext,
}

impl From<Ciphertext> for EncryptedDsMessage {
    fn from(payload: Ciphertext) -> Self {
        Self { payload }
    }
}

impl From<Vec<u8>> for DsFanoutPayload {
    fn from(assisted_message: Vec<u8>) -> Self {
        Self {
            payload: assisted_message,
        }
    }
}

impl AsRef<Ciphertext> for EncryptedDsMessage {
    fn as_ref(&self) -> &Ciphertext {
        &self.payload
    }
}

impl EarEncryptable<RatchetKey, EncryptedDsMessage> for DsFanoutPayload {}

/// This is the pseudonymous client id used on the DS.
#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub(crate) struct DsClientId {
    id: Vec<u8>,
}

// === DS ===

#[derive(TlsDeserialize, TlsSize, ToSchema)]
pub struct CreateGroupParams {
    pub group_id: GroupId,
    pub leaf_node: LeafNode,
    pub encrypted_credential_chain: EncryptedCredentialChain,
    pub creator_client_reference: QsClientReference,
    pub creator_user_auth_key: UserAuthKey,
    pub group_info: VerifiableGroupInfo,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct UpdateQsClientReferenceParams {
    group_id: GroupId,
    sender: LeafNodeIndex,
    new_queue_config: QsClientReference,
}

impl UpdateQsClientReferenceParams {
    pub fn sender(&self) -> LeafNodeIndex {
        self.sender
    }

    pub fn new_queue_config(&self) -> &QsClientReference {
        &self.new_queue_config
    }
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct WelcomeInfoParams {
    pub group_id: GroupId,
    pub sender: LeafSignatureKey,
    pub epoch: GroupEpoch,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct GetWelcomeInfoResponse {
    public_tree: Option<Vec<LeafNode>>,
    credential_chains: Vec<u8>,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct ExternalCommitInfoParams {
    pub group_id: GroupId,
    pub sender: UserKeyHash,
}

#[derive(TlsDeserialize, TlsSize, ToSchema)]
pub struct AssistedMessagePlus {
    pub commit: AssistedMessage,
    pub commit_bytes: Vec<u8>,
}

#[derive(TlsDeserialize, TlsSize, ToSchema)]
pub struct AddUsersParams {
    pub commit: AssistedMessagePlus,
    pub sender: UserKeyHash,
    pub welcome: AssistedWelcome,
    pub encrypted_welcome_attribution_infos: Vec<Vec<u8>>,
    pub key_package_batches: Vec<VerifiableKeyPackageBatch>,
}

impl AddUsersParams {
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, tls_codec::Error> {
        let bytes_copy = bytes;
        let (mut remaining_bytes, commit) = AssistedMessage::try_from_bytes(bytes)?;
        let commit_bytes = bytes_copy[0..bytes_copy.len() - remaining_bytes.len()].to_vec();
        let sender = UserKeyHash::tls_deserialize(&mut remaining_bytes)?;
        let welcome = AssistedWelcome::tls_deserialize(&mut remaining_bytes)?;
        let encrypted_welcome_attribution_infos =
            Vec::<Vec<u8>>::tls_deserialize(&mut remaining_bytes)?;
        let key_package_batches =
            Vec::<VerifiableKeyPackageBatch>::tls_deserialize(&mut remaining_bytes)?;
        Ok(Self {
            commit: AssistedMessagePlus {
                commit,
                commit_bytes,
            },
            sender,
            welcome,
            encrypted_welcome_attribution_infos,
            key_package_batches,
        })
    }
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct AddUsersParamsAad {
    pub encrypted_credential_information: Vec<Vec<u8>>,
}

#[derive(TlsDeserialize, TlsSize, ToSchema)]
pub struct RemoveUsersParams {
    pub commit: AssistedMessagePlus,
    pub sender: UserKeyHash,
}

impl RemoveUsersParams {
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, tls_codec::Error> {
        let bytes_copy = bytes;
        let (mut remaining_bytes, commit) = AssistedMessage::try_from_bytes(bytes)?;
        let commit_bytes = bytes_copy[0..bytes_copy.len() - remaining_bytes.len()].to_vec();
        let sender = UserKeyHash::tls_deserialize(&mut remaining_bytes)?;
        Ok(Self {
            commit: AssistedMessagePlus {
                commit,
                commit_bytes,
            },
            sender,
        })
    }
}

#[derive(TlsDeserialize, TlsSize, ToSchema)]
pub struct UpdateClientParams {
    pub commit: AssistedMessagePlus,
    pub sender: UserKeyHash,
}

impl UpdateClientParams {
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, tls_codec::Error> {
        let bytes_copy = bytes;
        let (mut remaining_bytes, commit) = AssistedMessage::try_from_bytes(bytes)?;
        let commit_bytes = bytes_copy[0..bytes_copy.len() - remaining_bytes.len()].to_vec();
        let sender = UserKeyHash::tls_deserialize(&mut remaining_bytes)?;
        Ok(Self {
            commit: AssistedMessagePlus {
                commit,
                commit_bytes,
            },
            sender,
        })
    }
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct UpdateClientParamsAad {
    pub option_encrypted_credential_information: Option<EncryptedCredentialChain>,
}

#[derive(TlsDeserialize, TlsSize, ToSchema)]
pub struct JoinGroupParams {
    pub external_commit: AssistedMessagePlus,
    pub sender: UserKeyHash,
    pub qs_client_reference: QsClientReference,
}

impl JoinGroupParams {
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, tls_codec::Error> {
        let bytes_copy = bytes;
        let (mut remaining_bytes, commit) = AssistedMessage::try_from_bytes(bytes)?;
        let commit_bytes = bytes_copy[0..bytes_copy.len() - remaining_bytes.len()].to_vec();
        let sender = UserKeyHash::tls_deserialize(&mut remaining_bytes)?;
        let qs_client_reference = QsClientReference::tls_deserialize(&mut remaining_bytes)?;
        Ok(Self {
            external_commit: AssistedMessagePlus {
                commit,
                commit_bytes,
            },
            sender,
            qs_client_reference,
        })
    }
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct JoinGroupParamsAad {
    pub existing_user_clients: Vec<LeafNodeIndex>,
    pub encrypted_credential_information: EncryptedCredentialChain,
}

#[derive(TlsDeserialize, TlsSize, ToSchema)]
pub struct JoinConnectionGroupParams {
    pub external_commit: AssistedMessagePlus,
    pub sender: UserAuthKey,
    pub qs_client_reference: QsClientReference,
}

impl JoinConnectionGroupParams {
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, tls_codec::Error> {
        let bytes_copy = bytes;
        let (mut remaining_bytes, commit) = AssistedMessage::try_from_bytes(bytes)?;
        let commit_bytes = bytes_copy[0..bytes_copy.len() - remaining_bytes.len()].to_vec();
        let sender = UserAuthKey::tls_deserialize(&mut remaining_bytes)?;
        let qs_client_reference = QsClientReference::tls_deserialize(&mut remaining_bytes)?;
        Ok(Self {
            external_commit: AssistedMessagePlus {
                commit,
                commit_bytes,
            },
            sender,
            qs_client_reference,
        })
    }
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct JoinConnectionGroupParamsAad {
    pub encrypted_credential_information: EncryptedCredentialChain,
}

#[derive(TlsDeserialize, TlsSize, ToSchema)]
pub struct AddClientsParams {
    pub commit: AssistedMessagePlus,
    pub sender: UserKeyHash,
    pub welcome: AssistedWelcome,
    // TODO: Do we need those? They come from our own clients. We can probably
    // just send these through the all-clients group.
    pub encrypted_welcome_attribution_infos: Vec<u8>,
}

impl AddClientsParams {
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, tls_codec::Error> {
        let bytes_copy = bytes;
        let (mut remaining_bytes, commit) = AssistedMessage::try_from_bytes(bytes)?;
        let commit_bytes = bytes_copy[0..bytes_copy.len() - remaining_bytes.len()].to_vec();
        let sender = UserKeyHash::tls_deserialize(&mut remaining_bytes)?;
        let welcome = AssistedWelcome::tls_deserialize(&mut remaining_bytes)?;
        let encrypted_welcome_attribution_infos = Vec::<u8>::tls_deserialize(&mut remaining_bytes)?;
        Ok(Self {
            commit: AssistedMessagePlus {
                commit,
                commit_bytes,
            },
            sender,
            welcome,
            encrypted_welcome_attribution_infos,
        })
    }
}

#[derive(TlsDeserialize, TlsSize, ToSchema)]
pub struct AddClientsParamsAad {
    pub encrypted_credential_information: Vec<EncryptedCredentialChain>,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct RemoveClientsParams {
    commit: SerializedAssistedMessage,
    ear_key: GroupStateEarKey,
    user_auth_key: UserAuthKey,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct ResyncClientParams {
    external_commit: SerializedAssistedMessage,
    ear_key: GroupStateEarKey,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct SelfRemoveClientParams {
    remove_proposals: SerializedAssistedMessage,
    ear_key: GroupStateEarKey,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct SelfRemoveUserParams {
    remove_proposals: SerializedAssistedMessage,
    ear_key: GroupStateEarKey,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct SendMessageParams {
    application_message: SerializedAssistedMessage,
    ear_key: GroupStateEarKey,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct DeleteGroupParams {
    commit: SerializedAssistedMessage,
    ear_key: GroupStateEarKey,
}

/// This enum contains variatns for each DS endpoint.
#[repr(u8)]
pub(crate) enum DsRequestParams {
    AddUsers(AddUsersParams),
    RemoveUsers(RemoveUsersParams),
    WelcomeInfo(WelcomeInfoParams),
    ExternalCommitInfo(ExternalCommitInfoParams),
    CreateGroupParams(CreateGroupParams),
    UpdateQueueInfo(UpdateQsClientReferenceParams),
    UpdateClient(UpdateClientParams),
    JoinGroup(JoinGroupParams),
    JoinConnectionGroup(JoinConnectionGroupParams),
    AddClients(AddClientsParams),
}

impl DsRequestParams {
    pub(crate) fn group_id(&self) -> &GroupId {
        match self {
            DsRequestParams::AddUsers(add_user_params) => add_user_params.commit.commit.group_id(),
            DsRequestParams::WelcomeInfo(welcome_info_params) => &welcome_info_params.group_id,
            DsRequestParams::CreateGroupParams(create_group_params) => {
                &create_group_params.group_id
            }
            DsRequestParams::UpdateQueueInfo(update_queue_info_params) => {
                &update_queue_info_params.group_id
            }
            DsRequestParams::ExternalCommitInfo(external_commit_info_params) => {
                &external_commit_info_params.group_id
            }
            DsRequestParams::RemoveUsers(remove_users_params) => {
                remove_users_params.commit.commit.group_id()
            }
            DsRequestParams::UpdateClient(update_client_params) => {
                update_client_params.commit.commit.group_id()
            }
            DsRequestParams::JoinGroup(join_group_params) => {
                join_group_params.external_commit.commit.group_id()
            }
            DsRequestParams::JoinConnectionGroup(join_connection_group_params) => {
                join_connection_group_params
                    .external_commit
                    .commit
                    .group_id()
            }
            DsRequestParams::AddClients(add_clients_params) => {
                add_clients_params.commit.commit.group_id()
            }
        }
    }

    /// Returns a sender if the request contains a public message. Otherwise returns `None`.
    pub(crate) fn mls_sender(&self) -> Option<&Sender> {
        match self {
            DsRequestParams::AddUsers(add_users_params) => add_users_params.commit.commit.sender(),
            DsRequestParams::RemoveUsers(remove_users_params) => {
                remove_users_params.commit.commit.sender()
            }
            DsRequestParams::UpdateClient(update_client_params) => {
                update_client_params.commit.commit.sender()
            }
            DsRequestParams::JoinGroup(join_group_params) => {
                join_group_params.external_commit.commit.sender()
            }
            DsRequestParams::JoinConnectionGroup(join_connection_group_params) => {
                join_connection_group_params.external_commit.commit.sender()
            }
            DsRequestParams::AddClients(add_clients_params) => {
                add_clients_params.commit.commit.sender()
            }
            DsRequestParams::WelcomeInfo(_)
            | DsRequestParams::ExternalCommitInfo(_)
            | DsRequestParams::CreateGroupParams(_)
            | DsRequestParams::UpdateQueueInfo(_) => None,
        }
    }

    /// Returns a sender if the request contains a public message. Otherwise returns `None`.
    pub(crate) fn ds_sender(&self) -> DsSender {
        match self {
            DsRequestParams::AddUsers(add_users_params) => {
                DsSender::UserKeyHash(add_users_params.sender.clone())
            }
            DsRequestParams::WelcomeInfo(welcome_info_params) => {
                DsSender::LeafSignatureKey(welcome_info_params.sender.clone())
            }
            DsRequestParams::CreateGroupParams(create_group_params) => {
                DsSender::UserKeyHash(create_group_params.creator_user_auth_key.hash())
            }
            DsRequestParams::UpdateQueueInfo(update_queue_info_params) => {
                DsSender::LeafIndex(update_queue_info_params.sender)
            }
            DsRequestParams::ExternalCommitInfo(external_commit_info_params) => {
                DsSender::UserKeyHash(external_commit_info_params.sender.clone())
            }
            DsRequestParams::RemoveUsers(remove_users_params) => {
                DsSender::UserKeyHash(remove_users_params.sender.clone())
            }
            DsRequestParams::UpdateClient(update_client_params) => {
                DsSender::UserKeyHash(update_client_params.sender.clone())
            }
            DsRequestParams::JoinGroup(join_group_params) => {
                DsSender::UserKeyHash(join_group_params.sender.clone())
            }
            DsRequestParams::JoinConnectionGroup(join_connection_group_params) => {
                DsSender::UserKeyHash(join_connection_group_params.sender.hash())
            }
            DsRequestParams::AddClients(add_clients_params) => {
                DsSender::UserKeyHash(add_clients_params.sender.clone())
            }
        }
    }

    pub(crate) fn try_from_bytes(mut bytes: &[u8]) -> Result<Self, tls_codec::Error> {
        let mut reader = bytes;
        let params_type = u8::tls_deserialize(&mut reader)?;
        match params_type {
            0 => Ok(Self::AddUsers(AddUsersParams::try_from_bytes(bytes)?)),
            1 => Ok(Self::WelcomeInfo(WelcomeInfoParams::tls_deserialize(
                &mut bytes,
            )?)),
            2 => Ok(Self::ExternalCommitInfo(
                ExternalCommitInfoParams::tls_deserialize(&mut bytes)?,
            )),
            3 => Ok(Self::CreateGroupParams(CreateGroupParams::tls_deserialize(
                &mut bytes,
            )?)),
            4 => Ok(Self::UpdateQueueInfo(
                UpdateQsClientReferenceParams::tls_deserialize(&mut bytes)?,
            )),
            5 => Ok(Self::RemoveUsers(RemoveUsersParams::try_from_bytes(bytes)?)),
            6 => Ok(Self::UpdateClient(UpdateClientParams::try_from_bytes(
                bytes,
            )?)),
            7 => Ok(Self::JoinGroup(JoinGroupParams::try_from_bytes(bytes)?)),
            8 => Ok(Self::JoinConnectionGroup(
                JoinConnectionGroupParams::try_from_bytes(bytes)?,
            )),
            9 => Ok(Self::AddClients(AddClientsParams::try_from_bytes(bytes)?)),
            _ => Err(tls_codec::Error::InvalidInput),
        }
    }
}

#[derive(Clone, TlsSerialize, TlsDeserialize, TlsSize)]
#[repr(u8)]
pub enum DsSender {
    LeafIndex(LeafNodeIndex),
    LeafSignatureKey(LeafSignatureKey),
    UserKeyHash(UserKeyHash),
}

// TODO: this needs custom deserialization that ensures that the sender matches
// the request params.
pub(crate) struct ClientToDsMessageTbs {
    _version: MlsInfraVersion,
    group_state_ear_key: GroupStateEarKey,
    // This essentially includes the wire format.
    body: DsRequestParams,
}

impl ClientToDsMessageTbs {
    pub(crate) fn try_from_bytes(mut bytes: &[u8]) -> Result<Self, tls_codec::Error> {
        let version = MlsInfraVersion::tls_deserialize(&mut bytes)?;
        let group_state_ear_key = GroupStateEarKey::tls_deserialize(&mut bytes)?;
        let body = DsRequestParams::try_from_bytes(bytes)?;
        Ok(Self {
            _version: version,
            group_state_ear_key,
            body,
        })
    }

    fn sender(&self) -> DsSender {
        self.body.ds_sender()
    }
}

pub(crate) struct ClientToDsMessage {
    payload: ClientToDsMessageTbs,
    // Signature over all of the above.
    signature: Signature,
}

impl ClientToDsMessage {
    pub(crate) fn try_from_bytes(mut bytes: &[u8]) -> Result<Self, tls_codec::Error> {
        let payload = ClientToDsMessageTbs::try_from_bytes(bytes)?;
        let signature = Signature::tls_deserialize(&mut bytes)?;
        Ok(Self { payload, signature })
    }
}

pub struct VerifiableClientToDsMessage {
    message: ClientToDsMessage,
    serialized_payload: Vec<u8>,
}

impl VerifiableClientToDsMessage {
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, tls_codec::Error> {
        let all_bytes = bytes;
        let bytes_len_before = bytes.len();
        let message = ClientToDsMessage::try_from_bytes(bytes)?;
        let serialized_payload = all_bytes[..bytes_len_before - bytes.len()].to_vec();
        Ok(Self {
            message,
            serialized_payload,
        })
    }

    pub(crate) fn group_id(&self) -> &GroupId {
        self.message.payload.body.group_id()
    }

    pub(crate) fn ear_key(&self) -> &GroupStateEarKey {
        &self.message.payload.group_state_ear_key
    }

    pub(crate) fn sender(&self) -> DsSender {
        self.message.payload.sender()
    }

    /// If the message contains a group creation request, return a reference to
    /// the group creation parameters. Otherwise return None.
    ///
    /// Group creation messages are essentially self-authenticated, so it's okay
    /// to extract the content before verification.
    pub(crate) fn create_group_params(&self) -> Option<&CreateGroupParams> {
        match &self.message.payload.body {
            DsRequestParams::CreateGroupParams(group_creation_params) => {
                Some(group_creation_params)
            }
            _ => None,
        }
    }

    /// If the message contains a request to join a connection group, return the
    /// UserAuthKey. Requests to join connection groups are essentially
    /// self-authenticated, which is okay.
    pub(crate) fn join_connection_group_sender(&self) -> Option<&UserAuthKey> {
        match &self.message.payload.body {
            DsRequestParams::JoinConnectionGroup(params) => Some(&params.sender),
            _ => None,
        }
    }
}

impl Verifiable for VerifiableClientToDsMessage {
    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        Ok(self.serialized_payload.clone())
    }

    fn signature(&self) -> &Signature {
        &self.message.signature
    }

    fn label(&self) -> &str {
        "ClientToDsMessage"
    }
}

impl VerifiedStruct<VerifiableClientToDsMessage> for DsRequestParams {
    type SealingType = private_mod::Seal;

    fn from_verifiable(verifiable: VerifiableClientToDsMessage, _seal: Self::SealingType) -> Self {
        verifiable.message.payload.body
    }
}
