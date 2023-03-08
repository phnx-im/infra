//! This module contains structs and enums that represent messages that are
//! passed between clients and the backend.
//! TODO: We should eventually factor this module out, together with the crypto
//! module, to allow re-use by the client implementation.

use mls_assist::{
    messages::{AssistedMessage, AssistedWelcome, SerializedAssistedMessage},
    GroupEpoch, GroupId, LeafNode, LeafNodeIndex, Sender, VerifiableGroupInfo,
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
    },
    ds::{
        group_state::{EncryptedCredentialChain, UserKeyHash},
        WelcomeAttributionInfo,
    },
    qs::{QsClientReference, VerifiableKeyPackageBatch},
};

mod private_mod {
    #[derive(Default)]
    pub(crate) struct Seal;
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, Clone)]
pub struct ClientToClientMsg {
    pub assisted_message: Vec<u8>,
}

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
pub struct UpdateQueueInfoParams {
    group_id: GroupId,
    ear_key: GroupStateEarKey,
    new_queue_config: QsClientReference,
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
    group_id: GroupId,
    ear_key: GroupStateEarKey,
}

#[derive(TlsDeserialize, TlsSize, ToSchema)]
pub struct AddUsersParams {
    pub commit: AssistedMessage,
    pub commit_bytes: Vec<u8>,
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
            commit,
            commit_bytes,
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
    WelcomeInfo(WelcomeInfoParams),
    CreateGroupParams(CreateGroupParams),
}

impl RequestParams {
    pub(crate) fn group_id(&self) -> &GroupId {
        match self {
            RequestParams::AddUsers(add_user_params) => add_user_params.commit.group_id(),
            RequestParams::WelcomeInfo(welcom_info_params) => &welcom_info_params.group_id,
            RequestParams::CreateGroupParams(create_group_params) => &create_group_params.group_id,
        }
    }

    /// Returns a sender if the request contains a public message. Otherwise returns `None`.
    pub(crate) fn mls_sender(&self) -> Option<&Sender> {
        match self {
            RequestParams::AddUsers(add_user_params) => add_user_params.commit.sender(),
            RequestParams::WelcomeInfo(_) => None,
            RequestParams::CreateGroupParams(_) => None,
        }
    }

    /// Returns a sender if the request contains a public message. Otherwise returns `None`.
    pub(crate) fn ds_sender(&self) -> DsSender {
        match self {
            RequestParams::AddUsers(add_user_params) => {
                DsSender::UserKeyHash(add_user_params.sender.clone())
            }
            RequestParams::WelcomeInfo(welcome_info_params) => {
                DsSender::LeafSignatureKey(welcome_info_params.sender.clone())
            }
            RequestParams::CreateGroupParams(create_group_params) => {
                DsSender::UserKeyHash(create_group_params.creator_user_auth_key.hash())
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
    // This essentially includes the wire format.
    body: RequestParams,
}

impl ClientToDsMessageTbs {
    pub(crate) fn try_from_bytes(mut bytes: &[u8]) -> Result<Self, tls_codec::Error> {
        let version = MlsInfraVersion::tls_deserialize(&mut bytes)?;
        let group_state_ear_key = GroupStateEarKey::tls_deserialize(&mut bytes)?;
        let body = RequestParams::try_from_bytes(bytes)?;
        Ok(Self {
            version,
            group_state_ear_key,
            body,
        })
    }

    fn sender(&self) -> DsSender {
        self.body.ds_sender()
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
            RequestParams::AddUsers(_) | RequestParams::WelcomeInfo(_) => None,
            RequestParams::CreateGroupParams(group_creation_params) => Some(group_creation_params),
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

impl VerifiedStruct<VerifiableClientToDsMessage> for RequestParams {
    type SealingType = private_mod::Seal;

    fn from_verifiable(verifiable: VerifiableClientToDsMessage, _seal: Self::SealingType) -> Self {
        verifiable.message.payload.body
    }
}
