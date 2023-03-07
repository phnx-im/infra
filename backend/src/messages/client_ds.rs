//! This module contains structs and enums that represent messages that are
//! passed between clients and the backend.
//! TODO: We should eventually factor this module out, together with the crypto
//! module, to allow re-use by the client implementation.

use mls_assist::{
    messages::{AssistedMessage, AssistedWelcome, SerializedAssistedMessage},
    GroupId, LeafNode, LeafNodeIndex, Sender, SignaturePublicKey, VerifiableGroupInfo,
};
use tls_codec::{Deserialize, Size, TlsDeserialize, TlsSerialize, TlsSize};
use utoipa::ToSchema;

use crate::{
    crypto::{
        ear::keys::GroupStateEarKey,
        kdf::keys::RosterKdfKey,
        mac::{MacTag, TaggedStruct},
        signatures::{
            keys::{LeafSignatureKey, UserAuthKey},
            signable::{Signature, Verifiable, VerifiedStruct},
        },
        RatchetPublicKey,
    },
    ds::{
        group_state::{EncryptedCredentialChain, UserKeyHash},
        WelcomeAttributionInfo,
    },
    qs::{QsClientReference, UserId, VerifiableKeyPackageBatch},
};

use super::{AddPackage, FriendshipToken};

mod private_mod {
    #[derive(Default)]
    pub(crate) struct Seal;
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, Clone)]
pub struct ClientToClientMsg {
    pub assisted_message: Vec<u8>,
}

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
    pub creator_queue_config: QsClientReference,
    pub creator_user_auth_key: UserAuthKey,
    pub group_info: VerifiableGroupInfo,
    pub initial_ear_key: GroupStateEarKey,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct UpdateQueueInfoParams {
    group_id: GroupId,
    ear_key: GroupStateEarKey,
    new_queue_config: QsClientReference,
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

#[derive(TlsDeserialize, TlsSize, ToSchema)]
pub struct AddUsersParams {
    pub commit: AssistedMessage,
    pub commit_bytes: Vec<u8>,
    pub welcome: AssistedWelcome,
    pub encrypted_welcome_attribution_infos: Vec<Vec<u8>>,
    pub key_package_batches: Vec<VerifiableKeyPackageBatch>,
}

impl AddUsersParams {
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, tls_codec::Error> {
        let bytes_copy = bytes;
        let (mut remaining_bytes, commit) = AssistedMessage::try_from_bytes(bytes)?;
        let commit_bytes = bytes_copy[0..bytes_copy.len() - remaining_bytes.len()].to_vec();
        let welcome = AssistedWelcome::tls_deserialize(&mut remaining_bytes)?;
        let encrypted_welcome_attribution_infos =
            Vec::<Vec<u8>>::tls_deserialize(&mut remaining_bytes)?;
        let key_package_batches =
            Vec::<VerifiableKeyPackageBatch>::tls_deserialize(&mut remaining_bytes)?;
        Ok(Self {
            commit,
            welcome,
            encrypted_welcome_attribution_infos,
            key_package_batches,
            commit_bytes,
        })
    }
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
    user_id: UserId,
    user_record_auth_key: SignaturePublicKey,
    friendship_token: FriendshipToken,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct UserRecordParams {
    user_id: UserId,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct DeleteUserRecordParams {
    user_id: UserId,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct CreateClientRecordParams {
    client_record_auth_key: SignaturePublicKey,
    queue_encryption_key: RatchetPublicKey,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct UpdateClientRecordParams {
    client_id: DsClientId,
    client_record_auth_key: SignaturePublicKey,
    queue_encryption_key: RatchetPublicKey,
    blocklist_entries: Vec<GroupId>,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct ClientRecordParams {
    client_id: DsClientId,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct DeleteClientRecordParams {
    client_id: DsClientId,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct PublishKeyPackagesParams {
    client_id: DsClientId,
    add_packages: Vec<AddPackage>,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct ClientKeyPackageParams {
    client_id: DsClientId,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct KeyPackageBatchParams {
    client_id: DsClientId,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct DequeueMessagesParams {
    client_id: DsClientId,
    sequence_number_start: u64,
    max_message_number: u64,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct WsParams {
    client_id: DsClientId,
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
    new_queue_config: QsClientReference,
}

impl UpdateQueueConfigParams {
    pub fn sender(&self) -> LeafNodeIndex {
        self.sender
    }

    pub fn group_id(&self) -> &GroupId {
        &self.group_id
    }

    pub fn new_queue_config(&self) -> &QsClientReference {
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
#[derive(TlsDeserialize, TlsSize)]
#[repr(u8)]
pub(crate) enum RequestParams {
    AddUsers(AddUsersParams),
}

impl RequestParams {
    pub(crate) fn group_id(&self) -> &GroupId {
        match self {
            RequestParams::AddUsers(add_user_params) => add_user_params.commit.group_id(),
        }
    }

    /// Returns a sender if the request contains a public message. Otherwise returns `None`.
    pub(crate) fn sender(&self) -> Option<&Sender> {
        match self {
            RequestParams::AddUsers(add_user_params) => add_user_params.commit.sender(),
        }
    }

    pub(crate) fn try_from_bytes(bytes: &[u8]) -> Result<Self, tls_codec::Error> {
        let mut reader = bytes;
        let params_type = u8::tls_deserialize(&mut reader)?;
        match params_type {
            0 => Ok(Self::AddUsers(AddUsersParams::try_from_bytes(bytes)?)),
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

#[derive(TlsDeserialize, TlsSize)]
// TODO: this needs custom deserialization that ensures that the sender matches
// the request params.
pub(crate) struct ClientToDsMessageTbs {
    version: MlsInfraVersion,
    group_state_ear_key: GroupStateEarKey,
    sender: DsSender,
    // This essentially includes the wire format.
    body: RequestParams,
}

impl ClientToDsMessageTbs {
    pub(crate) fn try_from_bytes(mut bytes: &[u8]) -> Result<Self, tls_codec::Error> {
        let version = MlsInfraVersion::tls_deserialize(&mut bytes)?;
        let group_state_ear_key = GroupStateEarKey::tls_deserialize(&mut bytes)?;
        let sender = DsSender::tls_deserialize(&mut bytes)?;
        let body = RequestParams::try_from_bytes(bytes)?;
        Ok(Self {
            version,
            group_state_ear_key,
            sender,
            body,
        })
    }
}

#[derive(TlsDeserialize, TlsSize)]
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

#[derive(TlsDeserialize, TlsSize)]
// TODO: This needs custom TLS Codec functions.
pub struct VerifiableClientToDsMessage {
    message: ClientToDsMessage,
    serialized_payload: Vec<u8>,
}

impl VerifiableClientToDsMessage {
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, tls_codec::Error> {
        let reader = bytes;
        let message = ClientToDsMessage::try_from_bytes(reader)?;
        let serialized_payload = bytes[..message.payload.tls_serialized_len()].to_vec();
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

    pub(crate) fn sender(&self) -> &DsSender {
        &self.message.payload.sender
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

impl VerifiedStruct<VerifiableClientToDsMessage> for RequestParams {
    type SealingType = private_mod::Seal;

    fn from_verifiable(verifiable: VerifiableClientToDsMessage, _seal: Self::SealingType) -> Self {
        verifiable.message.payload.body
    }
}
