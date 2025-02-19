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
            group_info::VerifiableGroupInfo, GroupId, LeafNodeIndex, MlsMessageOut, RatchetTreeIn,
        },
        treesync::RatchetTree,
    },
};
use tls_codec::{Serialize, TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::{
    crypto::{
        ear::keys::{EncryptedIdentityLinkKey, GroupStateEarKey},
        signatures::signable::{Signable, Signature, SignedStruct},
    },
    identifiers::QsReference,
    time::TimeStamp,
};

use super::{
    client_ds::{
        ConnectionGroupInfoParams, ExternalCommitInfoParams, UpdateQsClientReferenceParams,
        WelcomeInfoParams,
    },
    welcome_attribution_info::EncryptedWelcomeAttributionInfo,
    MlsInfraVersion,
};

#[derive(TlsSize, TlsDeserializeBytes)]
pub struct ExternalCommitInfoIn {
    pub verifiable_group_info: VerifiableGroupInfo,
    pub ratchet_tree_in: RatchetTreeIn,
    pub encrypted_identity_link_keys: Vec<EncryptedIdentityLinkKey>,
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
    pub sender_index: LeafNodeIndex,
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

#[expect(clippy::large_enum_variant)]
#[derive(Debug, TlsSerialize, TlsSize)]
#[repr(u8)]
pub enum DsRequestParamsOut {
    CreateGroupParams(CreateGroupParamsOut),
    WelcomeInfo(WelcomeInfoParams),
    ExternalCommitInfo(ExternalCommitInfoParams),
    ConnectionGroupInfo(ConnectionGroupInfoParams),
    UpdateQsClientReference(UpdateQsClientReferenceParams),
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
    _version: MlsInfraVersion,
    group_state_ear_key: GroupStateEarKey,
    // This essentially includes the wire format.
    body: DsRequestParamsOut,
}

impl ClientToDsMessageTbsOut {
    pub fn new(group_state_ear_key: GroupStateEarKey, body: DsRequestParamsOut) -> Self {
        Self {
            _version: MlsInfraVersion::default(),
            group_state_ear_key,
            body,
        }
    }
}

#[expect(clippy::large_enum_variant)]
#[derive(Debug, TlsSerialize, TlsSize)]
#[repr(u8)]
pub enum DsMessageTypeOut {
    Group(ClientToDsMessageOut),
    NonGroup,
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
}

impl SignedStruct<ClientToDsMessageTbsOut> for ClientToDsMessageOut {
    fn from_payload(payload: ClientToDsMessageTbsOut, signature: Signature) -> Self {
        Self { payload, signature }
    }
}
