// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module contains structs and enums that represent messages that are
//! passed between clients and the backend.
//! TODO: We should eventually factor this module out, together with the crypto
//! module, to allow re-use by the client implementation.

use mls_assist::{
    messages::{AssistedMessage, AssistedWelcome},
    openmls::{
        prelude::{
            group_info::VerifiableGroupInfo, GroupEpoch, GroupId, LeafNodeIndex, MlsMessageIn,
            RatchetTreeIn, Sender, SignaturePublicKey,
        },
        treesync::RatchetTree,
    },
    openmls_traits::types::HpkeCiphertext,
};
use serde::{Deserialize, Serialize};
use tls_codec::{
    DeserializeBytes, Serialize as TlsSerializeTrait, TlsDeserializeBytes, TlsSerialize, TlsSize,
};
use utoipa::ToSchema;

use crate::{
    crypto::{
        ear::{
            keys::{GroupStateEarKey, RatchetKey},
            EarDecryptable, EarEncryptable, GenericDeserializable, GenericSerializable,
        },
        hpke::{
            HpkeDecryptable, HpkeEncryptable, JoinerInfoDecryptionKey, JoinerInfoEncryptionKey,
        },
        signatures::{
            keys::UserAuthVerifyingKey,
            signable::{Signature, Verifiable, VerifiedStruct},
        },
        QueueRatchet,
    },
    ds::{
        group_state::{EncryptedClientCredential, UserKeyHash},
        EncryptedWelcomeAttributionInfo,
    },
    qs::{KeyPackageBatch, QsClientReference, UNVERIFIED},
};

use super::{EncryptedQsQueueMessage, MlsInfraVersion};

mod private_mod {
    #[derive(Default)]
    pub(crate) struct Seal;
}

/// This is the pseudonymous client id used on the DS.
#[derive(TlsSerialize, TlsDeserializeBytes, TlsSize, ToSchema)]
pub(crate) struct DsClientId {
    id: Vec<u8>,
}

// === DS ===

pub type QsQueueRatchet = QueueRatchet<EncryptedQsQueueMessage, QsQueueMessagePayload>;

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Clone, Serialize, Deserialize)]
#[repr(u8)]
pub enum QsQueueMessageType {
    WelcomeBundle,
    MlsMessage,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Clone, Serialize, Deserialize)]
pub struct QsQueueMessagePayload {
    pub message_type: QsQueueMessageType,
    pub payload: Vec<u8>,
}

impl QsQueueMessagePayload {
    pub fn extract(self) -> Result<ExtractedQsQueueMessagePayload, tls_codec::Error> {
        let message = match self.message_type {
            QsQueueMessageType::WelcomeBundle => {
                let wb = WelcomeBundle::tls_deserialize_exact(&self.payload)?;
                ExtractedQsQueueMessagePayload::WelcomeBundle(wb)
            }
            QsQueueMessageType::MlsMessage => {
                let message = <MlsMessageIn as tls_codec::Deserialize>::tls_deserialize(
                    &mut self.payload.as_slice(),
                )?;
                ExtractedQsQueueMessagePayload::MlsMessage(message)
            }
        };
        Ok(message)
    }
}

pub enum ExtractedQsQueueMessagePayload {
    WelcomeBundle(WelcomeBundle),
    MlsMessage(MlsMessageIn),
}

impl TryFrom<WelcomeBundle> for QsQueueMessagePayload {
    type Error = tls_codec::Error;

    fn try_from(welcome_bundle: WelcomeBundle) -> Result<Self, Self::Error> {
        let payload = welcome_bundle.tls_serialize_detached()?;
        Ok(Self {
            message_type: QsQueueMessageType::WelcomeBundle,
            payload,
        })
    }
}

impl From<AssistedMessagePlus> for QsQueueMessagePayload {
    fn from(value: AssistedMessagePlus) -> Self {
        Self {
            message_type: QsQueueMessageType::MlsMessage,
            payload: value.message_bytes,
        }
    }
}

impl EarEncryptable<RatchetKey, EncryptedQsQueueMessage> for QsQueueMessagePayload {}
impl EarDecryptable<RatchetKey, EncryptedQsQueueMessage> for QsQueueMessagePayload {}

#[derive(TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct InfraAadMessage {
    version: MlsInfraVersion,
    payload: InfraAadPayload,
}

impl From<InfraAadPayload> for InfraAadMessage {
    fn from(payload: InfraAadPayload) -> Self {
        Self {
            version: MlsInfraVersion::default(),
            payload,
        }
    }
}

impl InfraAadMessage {
    pub fn version(&self) -> MlsInfraVersion {
        self.version
    }

    pub fn into_payload(self) -> InfraAadPayload {
        self.payload
    }
}

#[derive(TlsSerialize, TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub enum InfraAadPayload {
    AddUsers(AddUsersParamsAad),
    UpdateClient(UpdateClientParamsAad),
    JoinGroup(JoinGroupParamsAad),
    JoinConnectionGroup(JoinConnectionGroupParamsAad),
    AddClients(AddClientsParamsAad),
    RemoveUsers,
    RemoveClients,
    ResyncClient,
    DeleteGroup,
    // There is no SelfRemoveClient entry, since that message consists of a
    // single proposal and since we don't otherwise support individual
    // proposals, there is not need to signal it explicitly.
}

#[derive(
    PartialEq,
    Eq,
    Debug,
    Clone,
    Serialize,
    Deserialize,
    TlsSerialize,
    TlsDeserializeBytes,
    TlsSize,
    ToSchema,
)]
pub struct EventMessage {
    pub group_id: GroupId,
    pub sender_index: LeafNodeIndex,
    pub epoch: GroupEpoch,
    pub payload: Vec<u8>,
}

impl EventMessage {
    pub fn group_id(&self) -> &GroupId {
        &self.group_id
    }

    pub fn sender_index(&self) -> LeafNodeIndex {
        self.sender_index
    }

    pub fn epoch(&self) -> GroupEpoch {
        self.epoch
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
}

#[derive(Debug, TlsDeserializeBytes, TlsSize, ToSchema)]
pub struct CreateGroupParams {
    pub group_id: GroupId,
    pub leaf_node: RatchetTreeIn,
    pub encrypted_client_credential: EncryptedClientCredential,
    pub creator_client_reference: QsClientReference,
    pub creator_user_auth_key: UserAuthVerifyingKey,
    pub group_info: VerifiableGroupInfo,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, ToSchema)]
pub struct UpdateQsClientReferenceParams {
    pub group_id: GroupId,
    pub sender: LeafNodeIndex,
    pub new_queue_config: QsClientReference,
}

impl UpdateQsClientReferenceParams {
    pub fn sender(&self) -> LeafNodeIndex {
        self.sender
    }

    pub fn new_queue_config(&self) -> &QsClientReference {
        &self.new_queue_config
    }
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, ToSchema)]
pub struct WelcomeInfoParams {
    pub group_id: GroupId,
    // The Public key from the sender's InfraCredential
    pub sender: SignaturePublicKey,
    pub epoch: GroupEpoch,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, ToSchema)]
pub struct GetWelcomeInfoResponse {
    public_tree: Option<RatchetTreeIn>,
    credential_chains: Vec<u8>,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, ToSchema)]
pub struct ExternalCommitInfoParams {
    pub group_id: GroupId,
    pub sender: UserKeyHash,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, ToSchema)]
pub struct ConnectionGroupInfoParams {
    pub group_id: GroupId,
}

// TODO: We want this to contain the message bytes as well s.t. we don't have to
// re-serialize after processing on the server side. This proves to be tricky
// even though we now have DeserializeBytes.
#[derive(Debug, TlsDeserializeBytes, TlsSize, ToSchema)]
pub struct AssistedMessagePlus {
    pub message: AssistedMessage,
    pub message_bytes: Vec<u8>,
}

#[derive(Debug, TlsSize, TlsDeserializeBytes)]
pub struct AddUsersParams {
    pub commit: AssistedMessagePlus,
    pub sender: UserKeyHash,
    pub welcome: AssistedWelcome,
    pub encrypted_welcome_attribution_infos: Vec<EncryptedWelcomeAttributionInfo>,
    pub key_package_batches: Vec<KeyPackageBatch<UNVERIFIED>>,
}

#[derive(TlsSerialize, TlsDeserializeBytes, TlsSize, ToSchema)]
pub struct AddUsersParamsAad {
    pub encrypted_credential_information: Vec<EncryptedClientCredential>,
}

#[derive(Debug, TlsDeserializeBytes, TlsSize, ToSchema)]
pub struct RemoveUsersParams {
    pub commit: AssistedMessagePlus,
    pub sender: UserKeyHash,
}

#[derive(Debug, TlsDeserializeBytes, TlsSize, ToSchema)]
pub struct UpdateClientParams {
    pub commit: AssistedMessagePlus,
    pub sender: LeafNodeIndex,
    pub new_user_auth_key_option: Option<UserAuthVerifyingKey>,
}

#[derive(TlsSerialize, TlsDeserializeBytes, TlsSize, ToSchema)]
pub struct UpdateClientParamsAad {
    pub option_encrypted_credential_information: Option<EncryptedClientCredential>,
}

#[derive(Debug, TlsDeserializeBytes, TlsSize, ToSchema)]
pub struct JoinGroupParams {
    pub external_commit: AssistedMessagePlus,
    pub sender: UserKeyHash,
    pub qs_client_reference: QsClientReference,
}

#[derive(TlsSerialize, TlsDeserializeBytes, TlsSize, ToSchema)]
pub struct JoinGroupParamsAad {
    pub existing_user_clients: Vec<LeafNodeIndex>,
    pub encrypted_credential_information: EncryptedClientCredential,
}

#[derive(Debug, TlsDeserializeBytes, TlsSize, ToSchema)]
pub struct JoinConnectionGroupParams {
    pub external_commit: AssistedMessagePlus,
    pub sender: UserAuthVerifyingKey,
    pub qs_client_reference: QsClientReference,
}

#[derive(TlsSerialize, TlsDeserializeBytes, TlsSize, ToSchema)]
pub struct JoinConnectionGroupParamsAad {
    pub encrypted_credential_information: EncryptedClientCredential,
}

#[derive(Debug, TlsDeserializeBytes, TlsSize, ToSchema)]
pub struct AddClientsParams {
    pub commit: AssistedMessagePlus,
    pub sender: UserKeyHash,
    pub welcome: AssistedWelcome,
    // TODO: Do we need those? They come from our own clients. We can probably
    // just send these through the all-clients group.
    pub encrypted_welcome_attribution_infos: EncryptedWelcomeAttributionInfo,
}

#[derive(TlsSerialize, TlsDeserializeBytes, TlsSize, ToSchema)]
pub struct AddClientsParamsAad {
    pub encrypted_credential_information: Vec<EncryptedClientCredential>,
}

#[derive(Debug, TlsDeserializeBytes, TlsSize, ToSchema)]
pub struct RemoveClientsParams {
    pub commit: AssistedMessagePlus,
    pub sender: UserKeyHash,
    pub new_auth_key: UserAuthVerifyingKey,
}

#[derive(Debug, TlsDeserializeBytes, TlsSize, ToSchema)]
pub struct ResyncClientParams {
    pub external_commit: AssistedMessagePlus,
    pub sender: UserKeyHash,
}

#[derive(Debug, TlsDeserializeBytes, TlsSize, ToSchema)]
pub struct SelfRemoveClientParams {
    pub remove_proposal: AssistedMessagePlus,
    pub sender: UserKeyHash,
}

#[derive(Debug, TlsDeserializeBytes, TlsSize, ToSchema)]
pub struct SendMessageParams {
    pub message: AssistedMessagePlus,
    pub sender: LeafNodeIndex,
}

#[derive(Debug, TlsDeserializeBytes, TlsSize, ToSchema)]
pub struct DispatchEventParams {
    pub event: EventMessage,
    pub sender: LeafNodeIndex,
}

#[derive(Debug, TlsDeserializeBytes, TlsSize, ToSchema)]
pub struct DeleteGroupParams {
    pub commit: AssistedMessagePlus,
    pub sender: UserKeyHash,
}

/// This enum contains variants for each DS endpoint.
#[derive(Debug, TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub(crate) enum DsRequestParams {
    AddUsers(AddUsersParams),
    RemoveUsers(RemoveUsersParams),
    WelcomeInfo(WelcomeInfoParams),
    ExternalCommitInfo(ExternalCommitInfoParams),
    ConnectionGroupInfo(ConnectionGroupInfoParams),
    CreateGroupParams(CreateGroupParams),
    UpdateQsClientReference(UpdateQsClientReferenceParams),
    UpdateClient(UpdateClientParams),
    JoinGroup(JoinGroupParams),
    JoinConnectionGroup(JoinConnectionGroupParams),
    AddClients(AddClientsParams),
    RemoveClients(RemoveClientsParams),
    ResyncClient(ResyncClientParams),
    SelfRemoveClient(SelfRemoveClientParams),
    SendMessage(SendMessageParams),
    DeleteGroup(DeleteGroupParams),
    DispatchEvent(DispatchEventParams),
}

impl DsRequestParams {
    pub(crate) fn group_id(&self) -> &GroupId {
        match self {
            DsRequestParams::AddUsers(add_user_params) => add_user_params.commit.message.group_id(),
            DsRequestParams::WelcomeInfo(welcome_info_params) => &welcome_info_params.group_id,
            DsRequestParams::CreateGroupParams(create_group_params) => {
                &create_group_params.group_id
            }
            DsRequestParams::UpdateQsClientReference(update_queue_info_params) => {
                &update_queue_info_params.group_id
            }
            DsRequestParams::ExternalCommitInfo(external_commit_info_params) => {
                &external_commit_info_params.group_id
            }
            DsRequestParams::ConnectionGroupInfo(params) => &params.group_id,
            DsRequestParams::RemoveUsers(remove_users_params) => {
                remove_users_params.commit.message.group_id()
            }
            DsRequestParams::UpdateClient(update_client_params) => {
                update_client_params.commit.message.group_id()
            }
            DsRequestParams::JoinGroup(join_group_params) => {
                join_group_params.external_commit.message.group_id()
            }
            DsRequestParams::JoinConnectionGroup(join_connection_group_params) => {
                join_connection_group_params
                    .external_commit
                    .message
                    .group_id()
            }
            DsRequestParams::AddClients(add_clients_params) => {
                add_clients_params.commit.message.group_id()
            }
            DsRequestParams::RemoveClients(remove_clients_params) => {
                remove_clients_params.commit.message.group_id()
            }
            DsRequestParams::ResyncClient(resync_client_params) => {
                resync_client_params.external_commit.message.group_id()
            }
            DsRequestParams::SelfRemoveClient(self_remove_client_params) => {
                self_remove_client_params.remove_proposal.message.group_id()
            }
            DsRequestParams::SendMessage(send_message_params) => {
                send_message_params.message.message.group_id()
            }
            DsRequestParams::DeleteGroup(delete_group_params) => {
                delete_group_params.commit.message.group_id()
            }
            DsRequestParams::DispatchEvent(dispatch_event_params) => {
                dispatch_event_params.event.group_id()
            }
        }
    }

    /// Returns a sender if the request contains a public message. Otherwise returns `None`.
    pub(crate) fn mls_sender(&self) -> Option<&Sender> {
        match self {
            DsRequestParams::AddUsers(add_users_params) => add_users_params.commit.message.sender(),
            DsRequestParams::RemoveUsers(remove_users_params) => {
                remove_users_params.commit.message.sender()
            }
            DsRequestParams::UpdateClient(update_client_params) => {
                update_client_params.commit.message.sender()
            }
            DsRequestParams::JoinGroup(join_group_params) => {
                join_group_params.external_commit.message.sender()
            }
            DsRequestParams::JoinConnectionGroup(join_connection_group_params) => {
                join_connection_group_params
                    .external_commit
                    .message
                    .sender()
            }
            DsRequestParams::AddClients(add_clients_params) => {
                add_clients_params.commit.message.sender()
            }
            DsRequestParams::RemoveClients(remove_clients_params) => {
                remove_clients_params.commit.message.sender()
            }
            DsRequestParams::ResyncClient(resync_client_params) => {
                resync_client_params.external_commit.message.sender()
            }
            DsRequestParams::SelfRemoveClient(self_remove_client_params) => {
                self_remove_client_params.remove_proposal.message.sender()
            }
            DsRequestParams::DeleteGroup(delete_group_params) => {
                delete_group_params.commit.message.sender()
            }
            DsRequestParams::DispatchEvent(_) => {
                None
            }
            DsRequestParams::WelcomeInfo(_)
            | DsRequestParams::ExternalCommitInfo(_)
            | DsRequestParams::ConnectionGroupInfo(_)
            | DsRequestParams::CreateGroupParams(_)
            // Since we're leaking the leaf index in the header, we could
            // technically return the MLS sender here.
            | DsRequestParams::SendMessage(_)
            | DsRequestParams::UpdateQsClientReference(_) => None,
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
            DsRequestParams::UpdateQsClientReference(update_queue_info_params) => {
                DsSender::LeafIndex(update_queue_info_params.sender)
            }
            DsRequestParams::ExternalCommitInfo(external_commit_info_params) => {
                DsSender::UserKeyHash(external_commit_info_params.sender.clone())
            }
            DsRequestParams::RemoveUsers(remove_users_params) => {
                DsSender::UserKeyHash(remove_users_params.sender.clone())
            }
            DsRequestParams::UpdateClient(update_client_params) => {
                DsSender::LeafIndex(update_client_params.sender)
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
            DsRequestParams::RemoveClients(remove_clients_params) => {
                DsSender::UserKeyHash(remove_clients_params.sender.clone())
            }
            DsRequestParams::ResyncClient(resync_client_params) => {
                DsSender::UserKeyHash(resync_client_params.sender.clone())
            }
            DsRequestParams::SelfRemoveClient(self_remove_client_params) => {
                DsSender::UserKeyHash(self_remove_client_params.sender.clone())
            }
            DsRequestParams::SendMessage(send_message_params) => {
                DsSender::LeafIndex(send_message_params.sender)
            }
            DsRequestParams::DeleteGroup(delete_group_params) => {
                DsSender::UserKeyHash(delete_group_params.sender.clone())
            }
            DsRequestParams::DispatchEvent(dispatch_event_params) => {
                DsSender::LeafIndex(dispatch_event_params.event.sender_index())
            }
            DsRequestParams::ConnectionGroupInfo(_) => DsSender::Anonymous,
        }
    }
}

#[derive(Clone, TlsSerialize, TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub enum DsSender {
    LeafIndex(LeafNodeIndex),
    LeafSignatureKey(SignaturePublicKey),
    UserKeyHash(UserKeyHash),
    Anonymous,
}

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub(crate) struct ClientToDsMessageTbs {
    _version: MlsInfraVersion,
    group_state_ear_key: GroupStateEarKey,
    // This essentially includes the wire format.
    body: DsRequestParams,
}

impl ClientToDsMessageTbs {
    fn sender(&self) -> DsSender {
        self.body.ds_sender()
    }
}

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub(crate) struct ClientToDsMessageIn {
    payload: ClientToDsMessageTbs,
    // Signature over all of the above.
    signature: Signature,
}

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub struct VerifiableClientToDsMessage {
    message: ClientToDsMessageIn,
    serialized_payload: Vec<u8>,
}

impl VerifiableClientToDsMessage {
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
    pub(crate) fn join_connection_group_sender(&self) -> Option<&UserAuthVerifyingKey> {
        match &self.message.payload.body {
            DsRequestParams::JoinConnectionGroup(params) => Some(&params.sender),
            _ => None,
        }
    }

    /// This returns the payload without any verification. Can only be used with
    /// payloads that have an `Anonymous` sender.
    pub(crate) fn extract_without_verification(self) -> Option<DsRequestParams> {
        match self.message.payload.sender() {
            DsSender::Anonymous => Some(self.message.payload.body),
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

#[derive(TlsSerialize, TlsSize, Clone)]
pub struct DsJoinerInformation {
    pub group_state_ear_key: GroupStateEarKey,
    pub encrypted_client_credentials: Vec<Option<EncryptedClientCredential>>,
    pub ratchet_tree: RatchetTree,
}

impl GenericSerializable for DsJoinerInformation {
    type Error = tls_codec::Error;

    fn serialize(&self) -> Result<Vec<u8>, Self::Error> {
        self.tls_serialize_detached()
    }
}

#[derive(TlsSerialize, TlsDeserializeBytes, TlsSize, Clone)]
pub struct EncryptedDsJoinerInformation {
    pub ciphertext: HpkeCiphertext,
}

impl AsRef<HpkeCiphertext> for EncryptedDsJoinerInformation {
    fn as_ref(&self) -> &HpkeCiphertext {
        &self.ciphertext
    }
}

impl From<HpkeCiphertext> for EncryptedDsJoinerInformation {
    fn from(ciphertext: HpkeCiphertext) -> Self {
        Self { ciphertext }
    }
}

impl HpkeEncryptable<JoinerInfoEncryptionKey, EncryptedDsJoinerInformation>
    for DsJoinerInformation
{
}

#[derive(TlsDeserializeBytes, TlsSize, Clone)]
pub struct DsJoinerInformationIn {
    pub group_state_ear_key: GroupStateEarKey,
    pub encrypted_client_credentials: Vec<Option<EncryptedClientCredential>>,
    pub ratchet_tree: RatchetTreeIn,
}

impl HpkeDecryptable<JoinerInfoDecryptionKey, EncryptedDsJoinerInformation>
    for DsJoinerInformationIn
{
}

impl GenericDeserializable for DsJoinerInformationIn {
    type Error = tls_codec::Error;

    fn deserialize(bytes: &[u8]) -> Result<Self, Self::Error> {
        Self::tls_deserialize_exact(bytes)
    }
}

#[derive(TlsSerialize, TlsDeserializeBytes, TlsSize, Clone)]
pub struct WelcomeBundle {
    pub welcome: AssistedWelcome,
    // This is the part the DS shouldn't see.
    pub encrypted_attribution_info: EncryptedWelcomeAttributionInfo,
    // This part is added by the DS later.
    pub encrypted_joiner_info: EncryptedDsJoinerInformation,
}
