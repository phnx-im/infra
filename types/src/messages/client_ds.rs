// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module contains structs and enums that represent messages that are
//! passed between clients and the backend.
//! TODO: We should eventually factor this module out, together with the crypto
//! module, to allow re-use by the client implementation.

use mls_assist::{
    messages::{AssistedMessageIn, AssistedWelcome, SerializedMlsMessage},
    openmls::{
        prelude::{
            GroupEpoch, GroupId, LeafNodeIndex, MlsMessageIn, RatchetTreeIn, Sender,
            SignaturePublicKey,
        },
        treesync::RatchetTree,
    },
    openmls_traits::types::HpkeCiphertext,
};
use serde::{Deserialize, Serialize};
use tls_codec::{
    DeserializeBytes, Serialize as TlsSerializeTrait, Size, TlsDeserializeBytes, TlsSerialize,
    TlsSize,
};

use crate::{
    crypto::{
        ear::{
            keys::{EncryptedIdentityLinkKey, GroupStateEarKey, RatchetKey},
            EarDecryptable, EarEncryptable, GenericDeserializable, GenericSerializable,
        },
        hpke::{
            HpkeDecryptable, HpkeEncryptable, JoinerInfoDecryptionKey, JoinerInfoEncryptionKey,
        },
        ratchet::QueueRatchet,
        signatures::signable::{Signature, Verifiable, VerifiedStruct},
    },
    identifiers::QsReference,
    keypackage_batch::{KeyPackageBatch, UNVERIFIED},
    time::TimeStamp,
};

use super::{
    client_as::EncryptedFriendshipPackage,
    welcome_attribution_info::EncryptedWelcomeAttributionInfo, EncryptedQsQueueMessage,
    MlsInfraVersion,
};

mod private_mod {
    #[derive(Default)]
    pub struct Seal;
}

/// This is the pseudonymous client id used on the DS.
#[derive(TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub(crate) struct DsClientId {
    id: Vec<u8>,
}

// === DS ===

pub type QsQueueRatchet = QueueRatchet<EncryptedQsQueueMessage, QsQueueMessagePayload>;

#[derive(
    Debug, PartialEq, TlsSerialize, TlsDeserializeBytes, TlsSize, Clone, Serialize, Deserialize,
)]
#[repr(u8)]
pub enum QsQueueMessageType {
    WelcomeBundle,
    MlsMessage,
}

#[derive(
    Debug, PartialEq, TlsSerialize, TlsDeserializeBytes, TlsSize, Clone, Serialize, Deserialize,
)]
pub struct QsQueueMessagePayload {
    pub timestamp: TimeStamp,
    pub message_type: QsQueueMessageType,
    pub payload: Vec<u8>,
}

impl QsQueueMessagePayload {
    pub fn extract(self) -> Result<ExtractedQsQueueMessage, tls_codec::Error> {
        let payload = match self.message_type {
            QsQueueMessageType::WelcomeBundle => {
                let wb = WelcomeBundle::tls_deserialize_exact_bytes(&self.payload)?;
                ExtractedQsQueueMessagePayload::WelcomeBundle(wb)
            }
            QsQueueMessageType::MlsMessage => {
                let message = MlsMessageIn::tls_deserialize_exact_bytes(self.payload.as_slice())?;
                ExtractedQsQueueMessagePayload::MlsMessage(Box::new(message))
            }
        };
        Ok(ExtractedQsQueueMessage {
            timestamp: self.timestamp,
            payload,
        })
    }
}

#[derive(Debug)]
pub struct ExtractedQsQueueMessage {
    pub timestamp: TimeStamp,
    pub payload: ExtractedQsQueueMessagePayload,
}

#[derive(Debug)]
pub enum ExtractedQsQueueMessagePayload {
    WelcomeBundle(WelcomeBundle),
    MlsMessage(Box<MlsMessageIn>),
}

impl TryFrom<WelcomeBundle> for QsQueueMessagePayload {
    type Error = tls_codec::Error;

    fn try_from(welcome_bundle: WelcomeBundle) -> Result<Self, Self::Error> {
        let payload = welcome_bundle.tls_serialize_detached()?;
        Ok(Self {
            timestamp: TimeStamp::now(),
            message_type: QsQueueMessageType::WelcomeBundle,
            payload,
        })
    }
}

impl From<SerializedMlsMessage> for QsQueueMessagePayload {
    fn from(value: SerializedMlsMessage) -> Self {
        Self {
            timestamp: TimeStamp::now(),
            message_type: QsQueueMessageType::MlsMessage,
            payload: value.0,
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
    GroupOperation(GroupOperationParamsAad),
    Update(UpdateParamsAad),
    JoinGroup(JoinGroupParamsAad),
    JoinConnectionGroup(JoinConnectionGroupParamsAad),
    Resync,
    DeleteGroup,
    // There is no SelfRemoveClient entry, since that message consists of a
    // single proposal and since we don't otherwise support individual
    // proposals, there is not need to signal it explicitly.
}

#[derive(PartialEq, Eq, Debug, Clone, TlsSize, TlsSerialize, TlsDeserializeBytes)]
#[repr(u8)]
pub enum QsWsMessage {
    QueueUpdate,
    Event(DsEventMessage),
}

#[derive(
    PartialEq, Eq, Debug, Clone, Serialize, Deserialize, TlsSerialize, TlsDeserializeBytes, TlsSize,
)]
pub struct DsEventMessage {
    pub group_id: GroupId,
    pub sender_index: LeafNodeIndex,
    pub epoch: GroupEpoch,
    // Timestamp set by the DS at the time of processing the message.
    pub timestamp: TimeStamp,
    pub payload: Vec<u8>,
}

impl DsEventMessage {
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

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub struct CreateGroupParams {
    pub group_id: GroupId,
    pub leaf_node: RatchetTreeIn,
    pub encrypted_identity_link_key: EncryptedIdentityLinkKey,
    pub creator_qs_reference: QsReference,
    pub group_info: MlsMessageIn,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct UpdateQsClientReferenceParams {
    pub group_id: GroupId,
    pub sender: LeafNodeIndex,
    pub new_qs_reference: QsReference,
}

impl UpdateQsClientReferenceParams {
    pub fn sender(&self) -> LeafNodeIndex {
        self.sender
    }

    pub fn new_qs_reference(&self) -> &QsReference {
        &self.new_qs_reference
    }
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct WelcomeInfoParams {
    pub group_id: GroupId,
    // The Public key from the sender's PseudonymousCredential
    pub sender: SignaturePublicKey,
    pub epoch: GroupEpoch,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct GetWelcomeInfoResponse {
    public_tree: Option<RatchetTreeIn>,
    credential_chains: Vec<u8>,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct ExternalCommitInfoParams {
    pub group_id: GroupId,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct ConnectionGroupInfoParams {
    pub group_id: GroupId,
}

#[derive(Debug, TlsSize, TlsDeserializeBytes)]
pub struct AddUsersInfo {
    pub welcome: AssistedWelcome,
    pub encrypted_welcome_attribution_infos: Vec<EncryptedWelcomeAttributionInfo>,
    pub key_package_batches: Vec<KeyPackageBatch<UNVERIFIED>>,
}

#[derive(Debug, TlsSize, TlsDeserializeBytes)]
pub struct GroupOperationParams {
    pub commit: AssistedMessageIn,
    pub add_users_info_option: Option<AddUsersInfo>,
}

#[derive(TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct CredentialUpdate {
    pub encrypted_identity_link_key: EncryptedIdentityLinkKey,
}

#[derive(TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct GroupOperationParamsAad {
    pub new_encrypted_identity_link_keys: Vec<EncryptedIdentityLinkKey>,
    pub credential_update_option: Option<CredentialUpdate>,
}

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub struct UpdateParams {
    pub commit: AssistedMessageIn,
}

#[derive(TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct UpdateParamsAad {
    pub option_encrypted_identity_link_key: Option<EncryptedIdentityLinkKey>,
}

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub struct JoinGroupParams {
    pub external_commit: AssistedMessageIn,
    pub qs_reference: QsReference,
}

#[derive(TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct JoinGroupParamsAad {
    pub encrypted_identity_link_key: EncryptedIdentityLinkKey,
}

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub struct JoinConnectionGroupParams {
    pub external_commit: AssistedMessageIn,
    pub qs_client_reference: QsReference,
}

#[derive(TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct JoinConnectionGroupParamsAad {
    pub encrypted_identity_link_key: EncryptedIdentityLinkKey,
    pub encrypted_friendship_package: EncryptedFriendshipPackage,
}

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub struct ResyncParams {
    pub external_commit: AssistedMessageIn,
}

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub struct SelfRemoveParams {
    pub remove_proposal: AssistedMessageIn,
}

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub struct SendMessageParams {
    pub message: AssistedMessageIn,
    pub sender: LeafNodeIndex,
}

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub struct DispatchEventParams {
    pub event: DsEventMessage,
    pub sender: LeafNodeIndex,
}

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub struct DeleteGroupParams {
    pub commit: AssistedMessageIn,
}

/// This enum contains variants for each DS endpoint.
#[expect(clippy::large_enum_variant)]
#[derive(Debug, TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub enum DsRequestParams {
    CreateGroupParams(CreateGroupParams),
    WelcomeInfo(WelcomeInfoParams),
    ExternalCommitInfo(ExternalCommitInfoParams),
    ConnectionGroupInfo(ConnectionGroupInfoParams),
    UpdateQsClientReference(UpdateQsClientReferenceParams),
    Update(UpdateParams),
    JoinGroup(JoinGroupParams),
    JoinConnectionGroup(JoinConnectionGroupParams),
    Resync(ResyncParams),
    SelfRemove(SelfRemoveParams),
    SendMessage(SendMessageParams),
    DeleteGroup(DeleteGroupParams),
    GroupOperation(GroupOperationParams),
    DispatchEvent(DispatchEventParams),
}

impl DsRequestParams {
    pub(crate) fn group_id(&self) -> &GroupId {
        match self {
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
            DsRequestParams::Update(update_client_params) => update_client_params.commit.group_id(),
            DsRequestParams::JoinGroup(join_group_params) => {
                join_group_params.external_commit.group_id()
            }
            DsRequestParams::JoinConnectionGroup(join_connection_group_params) => {
                join_connection_group_params.external_commit.group_id()
            }
            DsRequestParams::Resync(resync_client_params) => {
                resync_client_params.external_commit.group_id()
            }
            DsRequestParams::SelfRemove(self_remove_client_params) => {
                self_remove_client_params.remove_proposal.group_id()
            }
            DsRequestParams::SendMessage(send_message_params) => {
                send_message_params.message.group_id()
            }
            DsRequestParams::DeleteGroup(delete_group_params) => {
                delete_group_params.commit.group_id()
            }
            DsRequestParams::DispatchEvent(dispatch_event_params) => {
                dispatch_event_params.event.group_id()
            }
            DsRequestParams::GroupOperation(group_operation_params) => {
                group_operation_params.commit.group_id()
            }
        }
    }

    /// Returns a sender if the request contains a public message. Otherwise returns `None`.
    pub fn mls_sender(&self) -> Option<&Sender> {
        match self {
            DsRequestParams::Update(update_client_params) => {
                update_client_params.commit.sender()
            }
            DsRequestParams::JoinGroup(join_group_params) => {
                join_group_params.external_commit.sender()
            }
            DsRequestParams::JoinConnectionGroup(join_connection_group_params) => {
                join_connection_group_params
                    .external_commit
                    .sender()
            }
            DsRequestParams::Resync(resync_client_params) => {
                resync_client_params.external_commit.sender()
            }
            DsRequestParams::SelfRemove(self_remove_client_params) => {
                self_remove_client_params.remove_proposal.sender()
            }
            DsRequestParams::DeleteGroup(delete_group_params) => {
                delete_group_params.commit.sender()
            }
            DsRequestParams::GroupOperation(group_operation_params) => {
                group_operation_params.commit.sender()
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
    pub fn ds_sender(&self) -> DsSender {
        match self {
            DsRequestParams::WelcomeInfo(welcome_info_params) => {
                DsSender::LeafSignatureKey(welcome_info_params.sender.clone())
            }
            DsRequestParams::CreateGroupParams(_create_group_params) => DsSender::Anonymous,
            DsRequestParams::UpdateQsClientReference(update_queue_info_params) => {
                DsSender::LeafIndex(update_queue_info_params.sender)
            }
            DsRequestParams::ExternalCommitInfo(_external_commit_info_params) => {
                DsSender::Anonymous
            }
            DsRequestParams::Update(_update_client_params) => DsSender::Anonymous,
            DsRequestParams::JoinGroup(_join_group_params) => DsSender::Anonymous,
            DsRequestParams::JoinConnectionGroup(_join_connection_group_params) => {
                DsSender::Anonymous
            }
            DsRequestParams::Resync(_resync_client_params) => DsSender::Anonymous,
            DsRequestParams::SelfRemove(_self_remove_client_params) => DsSender::Anonymous,
            DsRequestParams::SendMessage(send_message_params) => {
                DsSender::LeafIndex(send_message_params.sender)
            }
            DsRequestParams::DeleteGroup(_delete_group_params) => DsSender::Anonymous,
            DsRequestParams::GroupOperation(_group_operation_params) => DsSender::Anonymous,
            DsRequestParams::DispatchEvent(dispatch_event_params) => {
                DsSender::LeafIndex(dispatch_event_params.event.sender_index())
            }
            DsRequestParams::ConnectionGroupInfo(_) => DsSender::Anonymous,
        }
    }
}

#[derive(Debug, Clone, TlsSerialize, TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub enum DsSender {
    LeafIndex(LeafNodeIndex),
    LeafSignatureKey(SignaturePublicKey),
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

#[expect(clippy::large_enum_variant)]
#[derive(Debug, TlsSize, TlsDeserializeBytes)]
#[repr(u8)]
pub enum DsMessageTypeIn {
    Group(VerifiableClientToDsMessage),
    NonGroup,
}

#[derive(Debug, TlsSize)]
pub struct VerifiableClientToDsMessage {
    message: ClientToDsMessageIn,
    serialized_payload: Vec<u8>,
}

impl DeserializeBytes for VerifiableClientToDsMessage {
    fn tls_deserialize_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error>
    where
        Self: Sized,
    {
        let (message, remainder) = ClientToDsMessageIn::tls_deserialize_bytes(bytes)?;
        // We want the payload to be the TBS bytes, which means we want all the bytes except the signature.
        let serialized_payload = bytes
            .get(..bytes.len() - remainder.len() - message.signature.tls_serialized_len())
            .ok_or(tls_codec::Error::EndOfStream)?
            .to_vec();
        let verifiable_message = Self {
            message,
            serialized_payload,
        };
        Ok((verifiable_message, remainder))
    }
}

impl VerifiableClientToDsMessage {
    pub fn group_id(&self) -> &GroupId {
        self.message.payload.body.group_id()
    }

    pub fn ear_key(&self) -> &GroupStateEarKey {
        &self.message.payload.group_state_ear_key
    }

    pub fn sender(&self) -> DsSender {
        self.message.payload.sender()
    }

    /// If the message contains a group creation request, return a reference to
    /// the group creation parameters. Otherwise return None.
    ///
    /// Group creation messages are essentially self-authenticated, so it's okay
    /// to extract the content before verification.
    pub fn create_group_params(&self) -> Option<&CreateGroupParams> {
        match &self.message.payload.body {
            DsRequestParams::CreateGroupParams(group_creation_params) => {
                Some(group_creation_params)
            }
            _ => None,
        }
    }

    /// This returns the payload without any verification. Can only be used with
    /// payloads that have an `Anonymous` sender.
    pub fn extract_without_verification(self) -> Option<DsRequestParams> {
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
    pub encrypted_identity_link_keys: Vec<EncryptedIdentityLinkKey>,
    pub ratchet_tree: RatchetTree,
}

impl GenericSerializable for DsJoinerInformation {
    type Error = tls_codec::Error;

    fn serialize(&self) -> Result<Vec<u8>, Self::Error> {
        self.tls_serialize_detached()
    }
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Clone)]
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
    pub encrypted_identity_link_keys: Vec<EncryptedIdentityLinkKey>,
    pub ratchet_tree: RatchetTreeIn,
}

impl HpkeDecryptable<JoinerInfoDecryptionKey, EncryptedDsJoinerInformation>
    for DsJoinerInformationIn
{
}

impl GenericDeserializable for DsJoinerInformationIn {
    type Error = tls_codec::Error;

    fn deserialize(bytes: &[u8]) -> Result<Self, Self::Error> {
        Self::tls_deserialize_exact_bytes(bytes)
    }
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Clone)]
pub struct WelcomeBundle {
    pub welcome: AssistedWelcome,
    // This is the part the DS shouldn't see.
    pub encrypted_attribution_info: EncryptedWelcomeAttributionInfo,
    // This part is added by the DS later.
    pub encrypted_joiner_info: EncryptedDsJoinerInformation,
}
