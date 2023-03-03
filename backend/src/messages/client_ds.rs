//! This module contains structs and enums that represent messages that are
//! passed between clients and the backend.
//! TODO: We should eventually factor this module out, together with the crypto
//! module, to allow re-use by the client implementation.

use mls_assist::{
    messages::SerializedAssistedMessage, GroupId, LeafNode, LeafNodeIndex, Sender,
    SignaturePublicKey, VerifiableGroupInfo,
};
use tls_codec::{Serialize, TlsDeserialize, TlsSerialize, TlsSize};
use utoipa::ToSchema;

use crate::{
    crypto::{
        ear::keys::GroupStateEarKey,
        kdf::keys::RosterKdfKey,
        mac::{MacTag, TaggedStruct},
        signatures::{
            keys::{LeafSignatureKey, UserAuthKey},
            signable::{Signable, Signature, Verifiable, VerifiedStruct},
        },
        RatchetPublicKey,
    },
    ds::{
        group_state::{EncryptedCredentialChain, UserKeyHash},
        WelcomeAttributionInfo,
    },
    qs::{ClientQueueConfig, KeyPackageBatch},
};

use super::{AddPackage, FriendshipToken, QsCid, QsUid};

mod private_mod {
    #[derive(Default)]
    pub(crate) struct Seal;
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, Clone)]
pub struct ClientToClientMsg {
    pub assisted_message: SerializedAssistedMessage,
}

// === DS ===

#[derive(TlsDeserialize, TlsSize, ToSchema)]
pub struct CreateGroupParams {
    pub group_id: GroupId,
    pub leaf_node: LeafNode,
    pub encrypted_credential_chain: EncryptedCredentialChain,
    pub creator_queue_config: ClientQueueConfig,
    pub creator_user_auth_key: UserAuthKey,
    pub group_info: VerifiableGroupInfo,
    pub initial_ear_key: GroupStateEarKey,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct UpdateQueueInfoParams {
    group_id: GroupId,
    ear_key: GroupStateEarKey,
    new_queue_config: ClientQueueConfig,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct WelcomeInfoParams {
    group_id: GroupId,
    ear_key: GroupStateEarKey,
    epoch: u64,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct GetWelcomeInfoResponse {
    public_tree: Option<Vec<LeafNode>>,
    credential_chains: Vec<u8>,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct ExternalCommitInfoParams {
    group_id: GroupId,
    ear_key: GroupStateEarKey,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct AddUsersParams {
    pub commit: SerializedAssistedMessage,
    pub serialized_assisted_welcome: Vec<u8>,
    pub welcome_attribution_info: Vec<WelcomeAttributionInfo>,
    pub key_package_batches: Vec<KeyPackageBatch>,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct AddUsersParamsAad {
    pub encrypted_credential_information: Vec<Vec<u8>>,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct RemoveUsersParams {
    commit: SerializedAssistedMessage,
    ear_key: GroupStateEarKey,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct UpdateClientParams {
    commit: SerializedAssistedMessage,
    ear_key: GroupStateEarKey,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct UpdateClientParamsAad {
    option_encrypted_credential_information: Option<Vec<u8>>,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct JoinGroupParams {
    external_commit: SerializedAssistedMessage,
    ear_key: GroupStateEarKey,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct JoinGroupParamsAad {
    existing_user_clients: Vec<LeafNodeIndex>,
    encrypted_credential_information: Vec<u8>,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct JoinConnectionGroupParams {
    external_commit: SerializedAssistedMessage,
    ear_key: GroupStateEarKey,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct JoinConnectionGroupParamsAad {
    encrypted_credential_information: Vec<u8>,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct AddClientsParams {
    commit: SerializedAssistedMessage,
    ear_key: GroupStateEarKey,
    serialized_assisted_welcome: Vec<u8>,
    welcome_attribution_info: WelcomeAttributionInfo,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct AddClientsParamsAad {
    encrypted_credential_information: Vec<u8>,
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

// === QS ===

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct CreateUserRecordParams {
    user_record_auth_key: SignaturePublicKey,
    friendship_token: FriendshipToken,
    client_record_auth_key: SignaturePublicKey,
    queue_encryption_key: RatchetPublicKey,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct UpdateUserRecordParams {
    qs_uid: QsUid,
    user_record_auth_key: SignaturePublicKey,
    friendship_token: FriendshipToken,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct UserRecordParams {
    qs_uid: QsUid,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct DeleteUserRecordParams {
    qs_uid: QsUid,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct CreateClientRecordParams {
    client_record_auth_key: SignaturePublicKey,
    queue_encryption_key: RatchetPublicKey,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct UpdateClientRecordParams {
    qs_cid: QsCid,
    client_record_auth_key: SignaturePublicKey,
    queue_encryption_key: RatchetPublicKey,
    blocklist_entries: Vec<GroupId>,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct ClientRecordParams {
    qs_cid: QsCid,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct DeleteClientRecordParams {
    qs_cid: QsCid,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct PublishKeyPackagesParams {
    qs_cid: QsCid,
    add_packages: Vec<AddPackage>,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct ClientKeyPackageParams {
    qs_cid: QsCid,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct KeyPackageBatchParams {
    qs_cid: QsCid,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct DequeueMessagesParams {
    qs_cid: QsCid,
    sequence_number_start: u64,
    max_message_number: u64,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct WsParams {
    qs_cid: QsCid,
}

// === Legacy ===

#[derive(TlsSerialize, TlsDeserialize, TlsSize)]
pub struct SendNonCommitParams {
    pub roster_key: RosterKdfKey,
    pub message: SerializedAssistedMessage, // Application message/Proposal
}

/// This is for the sender to create from SendNonCommitParams for serialization.
#[derive(TlsSerialize, TlsDeserialize, TlsSize)]
pub struct TaggedSendNonCommitParams {
    payload: SendNonCommitParams,
    mac: MacTag,
}

impl TaggedStruct<SendNonCommitParams> for TaggedSendNonCommitParams {
    fn from_untagged_payload(payload: SendNonCommitParams, mac: MacTag) -> Self {
        Self { payload, mac }
    }
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize)]
pub struct UpdateQueueConfigParams {
    roster_kdf_key: RosterKdfKey,
    group_id: GroupId,
    sender: LeafNodeIndex,
    new_queue_config: ClientQueueConfig,
}

impl UpdateQueueConfigParams {
    pub fn sender(&self) -> LeafNodeIndex {
        self.sender
    }

    pub fn group_id(&self) -> &GroupId {
        &self.group_id
    }

    pub fn new_queue_config(&self) -> &ClientQueueConfig {
        &self.new_queue_config
    }

    pub fn roster_kdf_key(&self) -> &RosterKdfKey {
        &self.roster_kdf_key
    }
}

/// Enum encoding the version of the MlsInfra protocol that was used to create
/// the given message.
#[derive(TlsSerialize, TlsDeserialize, TlsSize)]
#[repr(u8)]
pub(crate) enum MlsInfraVersion {
    Alpha,
}

/// This enum contains variatns for each DS endpoint.
#[derive(TlsSerialize, TlsDeserialize, TlsSize)]
#[repr(u8)]
pub(crate) enum RequestParams {
    AddUser(AddUsersParams),
}

impl RequestParams {
    pub(crate) fn group_id(&self) -> &GroupId {
        match self {
            // TODO: Waiting for OpenMLS' tls codec issue to get fixed.
            RequestParams::AddUser(add_user_params) => todo!(),
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

#[derive(TlsSerialize, TlsDeserialize, TlsSize)]
// TODO: this needs custom deserialization that ensures that the sender matches
// the request params.
pub(crate) struct ClientToDsMessageTbs {
    version: MlsInfraVersion,
    group_state_ear_key: GroupStateEarKey,
    sender: DsSender,
    // This essentially includes the wire format.
    body: RequestParams,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize)]
pub(crate) struct ClientToDsMessage {
    payload: ClientToDsMessageTbs,
    // Signature over all of the above.
    signature: Signature,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize)]
// TODO: This needs custom TLS Codec functions.
pub(crate) struct VerifiableClientToDsMessage {
    message: ClientToDsMessage,
    serialized_payload: Vec<u8>,
}

impl VerifiableClientToDsMessage {
    pub(crate) fn group_id(&self) -> &GroupId {
        self.message.payload.body.group_id()
    }

    pub(crate) fn ear_key(&self) -> &GroupStateEarKey {
        &self.message.payload.group_state_ear_key
    }

    pub(crate) fn sender(&self) -> &DsSender {
        &self.message.payload.sender
    }
}

impl Verifiable for VerifiableClientToDsMessage {
    fn unsigned_payload(&self) -> Result<&[u8], tls_codec::Error> {
        Ok(&self.serialized_payload)
    }

    fn signature(&self) -> &Signature {
        &self.message.signature
    }

    fn label(&self) -> &str {
        "ClientToDsMessage"
    }
}

impl VerifiedStruct<VerifiableClientToDsMessage> for RequestParams {
    type SealingType = private_mod::Seal;

    fn from_verifiable(verifiable: VerifiableClientToDsMessage, _seal: Self::SealingType) -> Self {
        verifiable.message.payload.body
    }
}

impl Signable for ClientToDsMessageTbs {
    type SignedOutput = ClientToDsMessage;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        "ClientToDsMessage"
    }
}
