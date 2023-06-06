// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module contains structs and enums that represent messages that are
//! passed between clients and the backend.
//! TODO: We should eventually factor this module out, together with the crypto
//! module, to allow re-use by the client implementation.

use mls_assist::{
    messages::AssistedWelcome,
    openmls::{
        prelude::{
            group_info::{GroupInfo, VerifiableGroupInfo},
            Extensions, GroupId, LeafNodeIndex, MlsMessageOut, RatchetTreeIn,
            Signature as MlsAssistSignature,
        },
        treesync::RatchetTree,
    },
};
use tls_codec::{Serialize, TlsDeserialize, TlsSerialize, TlsSize};

use crate::{
    crypto::{
        ear::keys::GroupStateEarKey,
        signatures::{
            keys::UserAuthVerifyingKey,
            signable::{Signable, Signature, SignedStruct},
        },
    },
    ds::{
        group_state::{EncryptedClientCredential, UserKeyHash},
        EncryptedWelcomeAttributionInfo,
    },
    qs::{KeyPackageBatch, QsClientReference, VERIFIED},
};

use super::{
    client_ds::{ExternalCommitInfoParams, UpdateQsClientReferenceParams, WelcomeInfoParams},
    MlsInfraVersion,
};

#[derive(TlsDeserialize, TlsSize)]
#[repr(u8)]
pub enum DsProcessResponseIn {
    Ok,
    WelcomeInfo(RatchetTreeIn),
    ExternalCommitInfo((VerifiableGroupInfo, RatchetTreeIn)),
}

#[derive(TlsSerialize, TlsSize)]
pub struct CreateGroupParamsOut {
    pub group_id: GroupId,
    pub leaf_node: RatchetTree,
    pub encrypted_credential_chain: EncryptedClientCredential,
    pub creator_client_reference: QsClientReference,
    pub creator_user_auth_key: UserAuthVerifyingKey,
    pub group_info: GroupInfo,
}

pub type AssistedMessagePlusOut = (MlsMessageOut, (MlsAssistSignature, Extensions));

#[derive(TlsSerialize, TlsSize)]
pub struct AddUsersParamsOut {
    // The commit and a partial assisted group info.
    pub commit: AssistedMessagePlusOut,
    pub sender: UserKeyHash,
    pub welcome: AssistedWelcome,
    pub encrypted_welcome_attribution_infos: Vec<EncryptedWelcomeAttributionInfo>,
    pub key_package_batches: Vec<KeyPackageBatch<VERIFIED>>,
}

#[derive(TlsSerialize, TlsSize)]
pub struct RemoveUsersParamsOut {
    pub commit: AssistedMessagePlusOut,
    pub sender: UserKeyHash,
}

#[derive(TlsSerialize, TlsSize)]
pub struct UpdateClientParamsOut {
    pub commit: AssistedMessagePlusOut,
    pub sender: LeafNodeIndex,
    pub new_user_auth_key_option: Option<UserAuthVerifyingKey>,
}

#[derive(TlsSerialize, TlsSize)]
pub struct JoinGroupParamsOut {
    pub external_commit: AssistedMessagePlusOut,
    pub sender: UserKeyHash,
    pub qs_client_reference: QsClientReference,
}

#[derive(TlsSerialize, TlsSize)]
pub struct JoinConnectionGroupParamsOut {
    pub external_commit: AssistedMessagePlusOut,
    pub sender: UserAuthVerifyingKey,
    pub qs_client_reference: QsClientReference,
}

#[derive(TlsSerialize, TlsSize)]
pub struct AddClientsParamsOut {
    pub commit: AssistedMessagePlusOut,
    pub sender: UserKeyHash,
    pub welcome: AssistedWelcome,
    // TODO: Do we need those? They come from our own clients. We can probably
    // just send these through the all-clients group.
    pub encrypted_welcome_attribution_infos: Vec<u8>,
}

#[derive(TlsSerialize, TlsSize)]
pub struct RemoveClientsParamsOut {
    pub commit: AssistedMessagePlusOut,
    pub sender: UserKeyHash,
    pub new_auth_key: UserAuthVerifyingKey,
}

#[derive(TlsSerialize, TlsSize)]
pub struct ResyncClientParamsOut {
    pub external_commit: AssistedMessagePlusOut,
    pub sender: UserKeyHash,
}

#[derive(TlsSerialize, TlsSize)]
pub struct SelfRemoveClientParamsOut {
    pub remove_proposal: AssistedMessagePlusOut,
    pub sender: UserKeyHash,
}

#[derive(TlsSerialize, TlsSize)]
pub struct SendMessageParamsOut {
    pub message: AssistedMessagePlusOut,
    pub sender: LeafNodeIndex,
}

#[derive(TlsSerialize, TlsSize)]
pub struct DeleteGroupParamsOut {
    pub commit: AssistedMessagePlusOut,
    pub sender: UserKeyHash,
}

#[derive(TlsSerialize, TlsSize)]
#[repr(u8)]
pub enum DsRequestParamsOut {
    AddUsers(AddUsersParamsOut),
    CreateGroupParams(CreateGroupParamsOut),
    RemoveUsers(RemoveUsersParamsOut),
    WelcomeInfo(WelcomeInfoParams),
    ExternalCommitInfo(ExternalCommitInfoParams),
    UpdateQsClientReference(UpdateQsClientReferenceParams),
    UpdateClient(UpdateClientParamsOut),
    JoinGroup(JoinGroupParamsOut),
    JoinConnectionGroup(JoinConnectionGroupParamsOut),
    AddClients(AddClientsParamsOut),
    RemoveClients(RemoveClientsParamsOut),
    ResyncClient(ResyncClientParamsOut),
    SelfRemoveClient(SelfRemoveClientParamsOut),
    SendMessage(SendMessageParamsOut),
    DeleteGroup(DeleteGroupParamsOut),
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

#[derive(TlsSerialize, TlsSize)]
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

#[derive(TlsSerialize, TlsSize)]
pub struct ClientToDsMessageOut {
    payload: ClientToDsMessageTbsOut,
    // Signature over all of the above.
    signature: Signature,
}

impl SignedStruct<ClientToDsMessageTbsOut> for ClientToDsMessageOut {
    fn from_payload(payload: ClientToDsMessageTbsOut, signature: Signature) -> Self {
        Self { payload, signature }
    }
}
