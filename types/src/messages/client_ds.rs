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
    TlsSize, TlsVarInt,
};

use crate::{
    crypto::{
        ear::{
            EarDecryptable, EarEncryptable, GenericDeserializable, GenericSerializable,
            keys::{
                EncryptedIdentityLinkKey, EncryptedUserProfileKey, GroupStateEarKey, RatchetKey,
            },
        },
        hpke::{HpkeDecryptable, HpkeEncryptable, JoinerInfoKeyType},
        ratchet::QueueRatchet,
        signatures::signable::{Signature, Verifiable, VerifiedStruct},
    },
    errors::version::VersionError,
    identifiers::QsReference,
    time::TimeStamp,
};

use super::{
    ApiVersion, EncryptedQsQueueMessageCtype, MlsInfraVersion,
    client_as::EncryptedFriendshipPackage,
    welcome_attribution_info::EncryptedWelcomeAttributionInfo,
};

mod private_mod {
    #[derive(Default)]
    pub struct Seal;
}

pub const CURRENT_DS_API_VERSION: ApiVersion = ApiVersion::new(1).unwrap();

pub const SUPPORTED_DS_API_VERSIONS: &[ApiVersion] = &[CURRENT_DS_API_VERSION];

/// This is the pseudonymous client id used on the DS.
#[derive(TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub(crate) struct DsClientId {
    id: Vec<u8>,
}

// === DS ===

pub type QsQueueRatchet = QueueRatchet<EncryptedQsQueueMessageCtype, QsQueueMessagePayload>;

#[derive(
    Debug, PartialEq, TlsSerialize, TlsDeserializeBytes, TlsSize, Clone, Serialize, Deserialize,
)]
#[repr(u8)]
pub enum QsQueueMessageType {
    WelcomeBundle,
    MlsMessage,
    UserProfileKeyUpdate,
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
            QsQueueMessageType::UserProfileKeyUpdate => {
                let message = UserProfileKeyUpdateParams::tls_deserialize_exact_bytes(
                    self.payload.as_slice(),
                )?;
                ExtractedQsQueueMessagePayload::UserProfileKeyUpdate(message)
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
    UserProfileKeyUpdate(UserProfileKeyUpdateParams),
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

impl TryFrom<&UserProfileKeyUpdateParams> for QsQueueMessagePayload {
    type Error = tls_codec::Error;

    fn try_from(params: &UserProfileKeyUpdateParams) -> Result<Self, Self::Error> {
        let payload = params.tls_serialize_detached()?;
        Ok(Self {
            timestamp: TimeStamp::now(),
            message_type: QsQueueMessageType::UserProfileKeyUpdate,
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

impl EarEncryptable<RatchetKey, EncryptedQsQueueMessageCtype> for QsQueueMessagePayload {}
impl EarDecryptable<RatchetKey, EncryptedQsQueueMessageCtype> for QsQueueMessagePayload {}

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
    JoinConnectionGroup(JoinConnectionGroupParamsAad),
    Resync,
    DeleteGroup,
    // There is no SelfRemoveClient entry, since that message consists of a
    // single proposal and since we don't otherwise support individual
    // proposals, there is not need to signal it explicitly.
}

#[derive(PartialEq, Eq, Debug, Clone, TlsSize, TlsSerialize, TlsDeserializeBytes)]
#[repr(u8)]
pub enum QsMessage {
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
    pub encrypted_user_profile_key: EncryptedUserProfileKey,
    pub creator_qs_reference: QsReference,
    pub group_info: MlsMessageIn,
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
    pub new_encrypted_user_profile_keys: Vec<EncryptedUserProfileKey>,
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
pub struct JoinConnectionGroupParams {
    pub external_commit: AssistedMessageIn,
    pub qs_client_reference: QsReference,
}

#[derive(TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct JoinConnectionGroupParamsAad {
    pub encrypted_identity_link_key: EncryptedIdentityLinkKey,
    pub encrypted_friendship_package: EncryptedFriendshipPackage,
    pub encrypted_user_profile_key: EncryptedUserProfileKey,
}

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub struct ResyncParams {
    pub external_commit: AssistedMessageIn,
    pub sender_index: LeafNodeIndex,
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

#[derive(Debug, Clone, TlsDeserializeBytes, TlsSize, TlsSerialize)]
pub struct UserProfileKeyUpdateParams {
    pub group_id: GroupId,
    pub sender_index: LeafNodeIndex,
    pub user_profile_key: EncryptedUserProfileKey,
}

#[derive(Debug)]
#[expect(clippy::large_enum_variant)]
pub enum DsVersionedRequestParams {
    Other(ApiVersion),
    Alpha(DsRequestParams),
}

impl DsVersionedRequestParams {
    pub fn version(&self) -> ApiVersion {
        match self {
            Self::Other(version) => *version,
            Self::Alpha(_) => ApiVersion::new(1).expect("infallible"),
        }
    }

    fn unversioned(&self) -> Result<&DsRequestParams, VersionError> {
        match self {
            Self::Alpha(params) => Ok(params),
            Self::Other(version) => Err(VersionError::new(*version, SUPPORTED_DS_API_VERSIONS)),
        }
    }

    pub fn into_unversioned(self) -> Result<(DsRequestParams, ApiVersion), VersionError> {
        let version = self.version();
        let params = match self {
            Self::Other(_) => {
                return Err(VersionError::new(version, SUPPORTED_DS_API_VERSIONS));
            }
            Self::Alpha(params) => params,
        };
        Ok((params, version))
    }
}

impl tls_codec::Size for DsVersionedRequestParams {
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

impl DeserializeBytes for DsVersionedRequestParams {
    fn tls_deserialize_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error> {
        let (version, bytes) = TlsVarInt::tls_deserialize_bytes(bytes)?;
        match version.value() {
            1 => {
                let (params, bytes) = DsRequestParams::tls_deserialize_bytes(bytes)?;
                Ok((Self::Alpha(params), bytes))
            }
            _ => Ok((Self::Other(ApiVersion::from_tls_value(version)), bytes)),
        }
    }
}

#[expect(clippy::large_enum_variant)]
#[derive(Debug, TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub enum DsRequestParams {
    Group {
        group_state_ear_key: GroupStateEarKey,
        request_params: DsGroupRequestParams,
    },
    NonGroup(DsNonGroupRequestParams),
}

/// This enum contains variants for each DS endpoint.
#[expect(clippy::large_enum_variant)]
#[derive(Debug, TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub enum DsGroupRequestParams {
    CreateGroupParams(CreateGroupParams),
    WelcomeInfo(WelcomeInfoParams),
    ExternalCommitInfo(ExternalCommitInfoParams),
    ConnectionGroupInfo(ConnectionGroupInfoParams),
    _UpdateQsClientReference,
    Update(UpdateParams),
    JoinConnectionGroup(JoinConnectionGroupParams),
    Resync(ResyncParams),
    SelfRemove(SelfRemoveParams),
    SendMessage(SendMessageParams),
    DeleteGroup(DeleteGroupParams),
    GroupOperation(GroupOperationParams),
    UserProfileKeyUpdate(UserProfileKeyUpdateParams),
    DispatchEvent(DispatchEventParams),
}

impl DsGroupRequestParams {
    pub(crate) fn group_id(&self) -> Option<&GroupId> {
        match self {
            Self::WelcomeInfo(welcome_info_params) => Some(&welcome_info_params.group_id),
            Self::CreateGroupParams(create_group_params) => Some(&create_group_params.group_id),
            Self::_UpdateQsClientReference => None,
            Self::ExternalCommitInfo(external_commit_info_params) => {
                Some(&external_commit_info_params.group_id)
            }
            Self::ConnectionGroupInfo(params) => Some(&params.group_id),
            Self::Update(update_client_params) => Some(update_client_params.commit.group_id()),
            Self::JoinConnectionGroup(join_connection_group_params) => {
                Some(join_connection_group_params.external_commit.group_id())
            }
            Self::Resync(resync_client_params) => {
                Some(resync_client_params.external_commit.group_id())
            }
            Self::SelfRemove(self_remove_client_params) => {
                Some(self_remove_client_params.remove_proposal.group_id())
            }
            Self::SendMessage(send_message_params) => Some(send_message_params.message.group_id()),
            Self::DeleteGroup(delete_group_params) => Some(delete_group_params.commit.group_id()),
            Self::DispatchEvent(dispatch_event_params) => {
                Some(dispatch_event_params.event.group_id())
            }
            Self::GroupOperation(group_operation_params) => {
                Some(group_operation_params.commit.group_id())
            }
            Self::UserProfileKeyUpdate(user_profile_update_params) => {
                Some(&user_profile_update_params.group_id)
            }
        }
    }

    /// Returns a sender if the request contains a public message. Otherwise returns `None`.
    pub fn mls_sender(&self) -> Option<&Sender> {
        match self {
            Self::Update(update_client_params) => {
                update_client_params.commit.sender()
            }
            Self::JoinConnectionGroup(join_connection_group_params) => {
                join_connection_group_params
                    .external_commit
                    .sender()
            }
            Self::Resync(resync_client_params) => {
                resync_client_params.external_commit.sender()
            }
            Self::SelfRemove(self_remove_client_params) => {
                self_remove_client_params.remove_proposal.sender()
            }
            Self::DeleteGroup(delete_group_params) => {
                delete_group_params.commit.sender()
            }
            Self::GroupOperation(group_operation_params) => {
                group_operation_params.commit.sender()
            }
            Self::DispatchEvent(_) => {
                None
            }
            Self::WelcomeInfo(_)
            | Self::ExternalCommitInfo(_)
            | Self::ConnectionGroupInfo(_)
            | Self::CreateGroupParams(_)
            // Since we're leaking the leaf index in the header, we could
            // technically return the MLS sender here.
            | Self::SendMessage(_)
            | Self::_UpdateQsClientReference
            | Self::UserProfileKeyUpdate(_) => None,
        }
    }

    /// Returns a sender if the request contains a public message. Otherwise returns `None`.
    pub fn ds_sender(&self) -> Option<DsSender> {
        match self {
            Self::WelcomeInfo(welcome_info_params) => Some(DsSender::LeafSignatureKey(
                welcome_info_params.sender.clone(),
            )),
            Self::_UpdateQsClientReference => None,
            Self::SendMessage(send_message_params) => {
                Some(DsSender::LeafIndex(send_message_params.sender))
            }
            Self::DispatchEvent(dispatch_event_params) => Some(DsSender::LeafIndex(
                dispatch_event_params.event.sender_index(),
            )),
            Self::UserProfileKeyUpdate(user_profile_update_params) => {
                Some(DsSender::LeafIndex(user_profile_update_params.sender_index))
            }
            // Messages that don't require additional auth
            Self::CreateGroupParams(_)
            | Self::ExternalCommitInfo(_)
            | Self::ConnectionGroupInfo(_)
            | Self::JoinConnectionGroup(_) => Some(DsSender::Anonymous),
            // Messages that require auth via the credential at the leaf, but
            // which contain an external commit
            Self::Resync(resync_params) => Some(DsSender::LeafIndex(resync_params.sender_index)),
            // Messages from which we pull the leaf index
            Self::DeleteGroup(_)
            | Self::GroupOperation(_)
            | Self::Update(_)
            | Self::SelfRemove(_) => self.mls_sender().and_then(|mls_sender| {
                if let Sender::Member(leaf_index) = mls_sender {
                    Some(DsSender::LeafIndex(*leaf_index))
                } else {
                    None
                }
            }),
        }
    }
}

#[derive(Debug, Clone, TlsSerialize, TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub enum DsSender {
    LeafIndex(LeafNodeIndex),
    ExternalSender(LeafNodeIndex),
    LeafSignatureKey(SignaturePublicKey),
    Anonymous,
}

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub(crate) struct ClientToDsMessageTbs {
    // This essentially includes the wire format.
    body: DsVersionedRequestParams,
}

impl ClientToDsMessageTbs {
    fn sender(&self) -> Option<DsSender> {
        match &self.body {
            DsVersionedRequestParams::Alpha(params) => match params {
                DsRequestParams::Group {
                    group_state_ear_key: _,
                    request_params,
                } => request_params.ds_sender(),
                DsRequestParams::NonGroup(params) => Some(params.ds_sender()),
            },
            DsVersionedRequestParams::Other(_) => None,
        }
    }
}

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub(crate) struct ClientToDsMessageIn {
    payload: ClientToDsMessageTbs,
    // Signature over all of the above.
    signature: Signature,
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
    pub fn group_id_and_ear_key(
        &self,
    ) -> Result<Option<(&GroupId, &GroupStateEarKey)>, VersionError> {
        self.message
            .payload
            .body
            .unversioned()
            .map(|params| match params {
                DsRequestParams::Group {
                    group_state_ear_key,
                    request_params,
                } => Some((request_params.group_id()?, group_state_ear_key)),
                DsRequestParams::NonGroup(_) => None,
            })
    }

    pub fn sender(&self) -> Option<DsSender> {
        self.message.payload.sender()
    }

    /// If the message contains a group creation request, return a reference to
    /// the group creation parameters. Otherwise return None.
    ///
    /// Group creation messages are essentially self-authenticated, so it's okay
    /// to extract the content before verification.
    pub fn create_group_params(&self) -> Result<Option<&CreateGroupParams>, VersionError> {
        match self.message.payload.body.unversioned()? {
            DsRequestParams::Group {
                group_state_ear_key: _,
                request_params: DsGroupRequestParams::CreateGroupParams(group_creation_params),
            } => Ok(Some(group_creation_params)),
            DsRequestParams::Group {
                group_state_ear_key: _,
                request_params: _,
            } => Ok(None),
            DsRequestParams::NonGroup(_) => Ok(None),
        }
    }

    /// This returns the payload without any verification. Can only be used with
    /// payloads that have an `Anonymous` sender.
    pub fn extract_without_verification(self) -> Option<DsVersionedRequestParams> {
        match self.message.payload.sender() {
            Some(DsSender::Anonymous) => Some(self.message.payload.body),
            _ => None,
        }
    }
}

impl Verifiable for VerifiableClientToDsMessage {
    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        Ok(self.serialized_payload.clone())
    }

    fn signature(&self) -> impl AsRef<[u8]> {
        &self.message.signature
    }

    fn label(&self) -> &str {
        "ClientToDsMessage"
    }
}

impl VerifiedStruct<VerifiableClientToDsMessage> for DsVersionedRequestParams {
    type SealingType = private_mod::Seal;

    fn from_verifiable(verifiable: VerifiableClientToDsMessage, _seal: Self::SealingType) -> Self {
        verifiable.message.payload.body
    }
}

#[derive(TlsSerialize, TlsSize, Clone)]
pub struct DsJoinerInformation {
    pub group_state_ear_key: GroupStateEarKey,
    pub encrypted_identity_link_keys: Vec<EncryptedIdentityLinkKey>,
    pub encrypted_user_profile_keys: Vec<EncryptedUserProfileKey>,
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

impl HpkeEncryptable<JoinerInfoKeyType, EncryptedDsJoinerInformation> for DsJoinerInformation {}

#[derive(TlsDeserializeBytes, TlsSize, Clone)]
pub struct DsJoinerInformationIn {
    pub group_state_ear_key: GroupStateEarKey,
    pub encrypted_identity_link_keys: Vec<EncryptedIdentityLinkKey>,
    pub encrypted_user_profile_keys: Vec<EncryptedUserProfileKey>,
    pub ratchet_tree: RatchetTreeIn,
}

impl HpkeDecryptable<JoinerInfoKeyType, EncryptedDsJoinerInformation> for DsJoinerInformationIn {}

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

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub enum DsNonGroupRequestParams {
    RequestGroupId,
}

impl DsNonGroupRequestParams {
    fn ds_sender(&self) -> DsSender {
        match self {
            Self::RequestGroupId => DsSender::Anonymous,
        }
    }
}
