// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module contains structs and enums that represent messages that are
//! passed between clients and the backend.
//! TODO: We should eventually factor this module out, together with the crypto
//! module, to allow re-use by the client implementation.

use mls_assist::{
    messages::AssistedMessageOut,
    openmls::{
        prelude::{
            GroupId, LeafNodeIndex, MlsMessageOut, RatchetTreeIn, group_info::VerifiableGroupInfo,
        },
        treesync::RatchetTree,
    },
};
use tls_codec::{Serialize, TlsDeserializeBytes, TlsSerialize, TlsSize, TlsVarInt};

use crate::{
    crypto::{
        ear::keys::{EncryptedIdentityLinkKey, GroupStateEarKey},
        signatures::signable::{Signable, Signature, SignedStruct},
    },
    errors::version::VersionError,
    identifiers::QsReference,
    time::TimeStamp,
};

use super::{
    ApiVersion,
    client_ds::{
        ConnectionGroupInfoParams, ExternalCommitInfoParams, SUPPORTED_DS_API_VERSIONS,
        WelcomeInfoParams,
    },
    welcome_attribution_info::EncryptedWelcomeAttributionInfo,
};

#[derive(TlsSize, TlsDeserializeBytes)]
pub struct ExternalCommitInfoIn {
    pub verifiable_group_info: VerifiableGroupInfo,
    pub ratchet_tree_in: RatchetTreeIn,
    pub encrypted_identity_link_keys: Vec<EncryptedIdentityLinkKey>,
}

#[expect(clippy::large_enum_variant)]
pub enum DsVersionedProcessResponseIn {
    Other(ApiVersion),
    Alpha(DsProcessResponseIn),
}

impl DsVersionedProcessResponseIn {
    pub fn version(&self) -> ApiVersion {
        match self {
            Self::Other(version) => *version,
            Self::Alpha(_) => ApiVersion::new(1).expect("infallible"),
        }
    }

    pub fn into_unversioned(self) -> Result<DsProcessResponseIn, VersionError> {
        match self {
            Self::Alpha(response) => Ok(response),
            Self::Other(version) => Err(VersionError::new(version, SUPPORTED_DS_API_VERSIONS)),
        }
    }
}

impl tls_codec::Size for DsVersionedProcessResponseIn {
    fn tls_serialized_len(&self) -> usize {
        match self {
            Self::Other(_) => self.version().tls_value().tls_serialized_len(),
            Self::Alpha(response) => {
                self.version().tls_value().tls_serialized_len() + response.tls_serialized_len()
            }
        }
    }
}

impl tls_codec::DeserializeBytes for DsVersionedProcessResponseIn {
    fn tls_deserialize_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error> {
        let (version, bytes) = TlsVarInt::tls_deserialize_bytes(bytes)?;
        match version.value() {
            1 => {
                let (response, bytes) = DsProcessResponseIn::tls_deserialize_bytes(bytes)?;
                Ok((Self::Alpha(response), bytes))
            }
            _ => Ok((Self::Other(ApiVersion::from_tls_value(version)), bytes)),
        }
    }
}

#[expect(clippy::large_enum_variant)]
#[derive(TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub enum DsProcessResponseIn {
    Ok,
    FanoutTimestamp(TimeStamp),
    WelcomeInfo(RatchetTreeIn),
    ExternalCommitInfo(ExternalCommitInfoIn),
    GroupId(GroupId),
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct CreateGroupParamsOut {
    pub group_id: GroupId,
    pub ratchet_tree: RatchetTree,
    pub encrypted_identity_link_key: EncryptedIdentityLinkKey,
    pub creator_client_reference: QsReference,
    pub group_info: MlsMessageOut,
}

#[derive(Debug, TlsSize, TlsSerialize)]
pub struct AddUsersInfoOut {
    pub welcome: MlsMessageOut,
    pub encrypted_welcome_attribution_infos: Vec<EncryptedWelcomeAttributionInfo>,
}

#[derive(Debug, TlsSize, TlsSerialize)]
pub struct GroupOperationParamsOut {
    pub commit: AssistedMessageOut,
    pub add_users_info_option: Option<AddUsersInfoOut>,
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct UpdateParamsOut {
    pub commit: AssistedMessageOut,
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct JoinConnectionGroupParamsOut {
    pub external_commit: AssistedMessageOut,
    pub qs_client_reference: QsReference,
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct ResyncParamsOut {
    pub external_commit: AssistedMessageOut,
    pub sender: LeafNodeIndex,
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct SelfRemoveParamsOut {
    pub remove_proposal: AssistedMessageOut,
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct SendMessageParamsOut {
    pub message: AssistedMessageOut,
    pub sender: LeafNodeIndex,
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct DeleteGroupParamsOut {
    pub commit: AssistedMessageOut,
}

#[derive(Debug)]
pub enum DsVersionedRequestParamsOut {
    Alpha(DsRequestParamsOut),
}

impl DsVersionedRequestParamsOut {
    pub fn with_version(
        params: DsRequestParamsOut,
        version: ApiVersion,
    ) -> Result<Self, VersionError> {
        match version.value() {
            1 => Ok(Self::Alpha(params)),
            _ => Err(VersionError::new(version, SUPPORTED_DS_API_VERSIONS)),
        }
    }

    pub fn change_version(
        self,
        to_version: ApiVersion,
    ) -> Result<(Self, ApiVersion), VersionError> {
        let from_version = self.version();
        match (to_version.value(), self) {
            (1, Self::Alpha(params)) => Ok((Self::Alpha(params), from_version)),
            (_, Self::Alpha(_)) => Err(VersionError::new(to_version, SUPPORTED_DS_API_VERSIONS)),
        }
    }

    pub(crate) fn version(&self) -> ApiVersion {
        match self {
            DsVersionedRequestParamsOut::Alpha(_) => ApiVersion::new(1).expect("infallible"),
        }
    }
}

impl tls_codec::Size for DsVersionedRequestParamsOut {
    fn tls_serialized_len(&self) -> usize {
        match self {
            DsVersionedRequestParamsOut::Alpha(params) => {
                self.version().tls_value().tls_serialized_len() + params.tls_serialized_len()
            }
        }
    }
}

// Note: Manual implementation because `TlsSerialize` does not support custom variant tags.
impl Serialize for DsVersionedRequestParamsOut {
    fn tls_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, tls_codec::Error> {
        match self {
            DsVersionedRequestParamsOut::Alpha(params) => {
                Ok(self.version().tls_value().tls_serialize(writer)?
                    + params.tls_serialize(writer)?)
            }
        }
    }
}

#[derive(Debug, TlsSerialize, TlsSize)]
#[repr(u8)]
#[expect(clippy::large_enum_variant)]
pub enum DsRequestParamsOut {
    Group {
        group_state_ear_key: GroupStateEarKey,
        request_params: DsGroupRequestParamsOut,
    },
    NonGroup(DsNonGroupRequestParamsOut),
}

#[expect(clippy::large_enum_variant)]
#[derive(Debug, TlsSerialize, TlsSize)]
#[repr(u8)]
pub enum DsGroupRequestParamsOut {
    CreateGroupParams(CreateGroupParamsOut),
    WelcomeInfo(WelcomeInfoParams),
    ExternalCommitInfo(ExternalCommitInfoParams),
    ConnectionGroupInfo(ConnectionGroupInfoParams),
    _UpdateQsClientReference,
    Update(UpdateParamsOut),
    JoinConnectionGroup(JoinConnectionGroupParamsOut),
    Resync(ResyncParamsOut),
    SelfRemove(SelfRemoveParamsOut),
    SendMessage(SendMessageParamsOut),
    DeleteGroup(DeleteGroupParamsOut),
    GroupOperation(GroupOperationParamsOut),
}

impl Signable for ClientToDsMessageTbsOut {
    type SignedOutput = ClientToDsMessageOut;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        "ClientToDsMessage"
    }
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct ClientToDsMessageTbsOut {
    // This essentially includes the wire format.
    body: DsVersionedRequestParamsOut,
}

impl ClientToDsMessageTbsOut {
    pub fn new(body: DsVersionedRequestParamsOut) -> Self {
        Self { body }
    }

    pub fn into_body(self) -> DsVersionedRequestParamsOut {
        self.body
    }
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct ClientToDsMessageOut {
    payload: ClientToDsMessageTbsOut,
    // Signature over all of the above.
    signature: Signature,
}

impl ClientToDsMessageOut {
    pub fn without_signature(payload: ClientToDsMessageTbsOut) -> Self {
        let signature = Signature::empty();
        Self { payload, signature }
    }

    pub fn into_payload(self) -> ClientToDsMessageTbsOut {
        self.payload
    }
}

impl SignedStruct<ClientToDsMessageTbsOut> for ClientToDsMessageOut {
    fn from_payload(payload: ClientToDsMessageTbsOut, signature: Signature) -> Self {
        Self { payload, signature }
    }
}

#[derive(Debug, TlsSerialize, TlsSize)]
#[repr(u8)]
pub enum DsNonGroupRequestParamsOut {
    RequestGroupId,
}
