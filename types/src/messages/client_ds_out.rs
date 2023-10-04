// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module contains structs and enums that represent messages that are
//! passed between clients and the backend.
//! TODO: We should eventually factor this module out, together with the crypto
//! module, to allow re-use by the client implementation.

use mls_assist::{
    messages::{AssistedMessageOut, AssistedWelcome},
    openmls::{
        prelude::{
            group_info::VerifiableGroupInfo, GroupId, LeafNodeIndex, MlsMessageOut, RatchetTreeIn,
        },
        treesync::RatchetTree,
    },
};
use tls_codec::{Serialize, TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::{
    credentials::EncryptedClientCredential,
    crypto::{
        ear::keys::{EncryptedSignatureEarKey, GroupStateEarKey},
        signatures::{
            keys::{UserAuthVerifyingKey, UserKeyHash},
            signable::{Signable, Signature, SignedStruct},
        },
    },
    identifiers::QsClientReference,
    keypackage_batch::{KeyPackageBatch, VERIFIED},
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
    pub encrypted_client_info: Vec<Option<(EncryptedClientCredential, EncryptedSignatureEarKey)>>,
}

#[derive(TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub enum DsProcessResponseIn {
    Ok,
    WelcomeInfo(RatchetTreeIn),
    ExternalCommitInfo(ExternalCommitInfoIn),
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct CreateGroupParamsOut {
    pub group_id: GroupId,
    pub ratchet_tree: RatchetTree,
    pub encrypted_client_credential: EncryptedClientCredential,
    pub encrypted_signature_ear_key: EncryptedSignatureEarKey,
    pub creator_client_reference: QsClientReference,
    pub creator_user_auth_key: UserAuthVerifyingKey,
    pub group_info: MlsMessageOut,
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct AddUsersParamsOut {
    pub commit: AssistedMessageOut,
    pub sender: UserKeyHash,
    pub welcome: MlsMessageOut,
    pub encrypted_welcome_attribution_infos: Vec<EncryptedWelcomeAttributionInfo>,
    pub key_package_batches: Vec<KeyPackageBatch<VERIFIED>>,
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct RemoveUsersParamsOut {
    pub commit: AssistedMessageOut,
    pub sender: UserKeyHash,
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct UpdateClientParamsOut {
    pub commit: AssistedMessageOut,
    pub sender: LeafNodeIndex,
    pub new_user_auth_key_option: Option<UserAuthVerifyingKey>,
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct JoinGroupParamsOut {
    pub external_commit: AssistedMessageOut,
    pub sender: UserKeyHash,
    pub qs_client_reference: QsClientReference,
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct JoinConnectionGroupParamsOut {
    pub external_commit: AssistedMessageOut,
    pub sender: UserAuthVerifyingKey,
    pub qs_client_reference: QsClientReference,
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct AddClientsParamsOut {
    pub commit: AssistedMessageOut,
    pub sender: UserKeyHash,
    pub welcome: AssistedWelcome,
    // TODO: Do we need those? They come from our own clients. We can probably
    // just send these through the all-clients group.
    pub encrypted_welcome_attribution_infos: Vec<EncryptedWelcomeAttributionInfo>,
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct RemoveClientsParamsOut {
    pub commit: AssistedMessageOut,
    pub sender: UserKeyHash,
    pub new_auth_key: UserAuthVerifyingKey,
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct ResyncClientParamsOut {
    pub external_commit: AssistedMessageOut,
    pub sender: UserKeyHash,
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct SelfRemoveClientParamsOut {
    pub remove_proposal: AssistedMessageOut,
    pub sender: UserKeyHash,
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct SendMessageParamsOut {
    pub message: AssistedMessageOut,
    pub sender: LeafNodeIndex,
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct DeleteGroupParamsOut {
    pub commit: AssistedMessageOut,
    pub sender: UserKeyHash,
}

#[derive(Debug, TlsSerialize, TlsSize)]
#[repr(u8)]
pub enum DsRequestParamsOut {
    AddUsers(AddUsersParamsOut),
    CreateGroupParams(CreateGroupParamsOut),
    RemoveUsers(RemoveUsersParamsOut),
    WelcomeInfo(WelcomeInfoParams),
    ExternalCommitInfo(ExternalCommitInfoParams),
    ConnectionGroupInfo(ConnectionGroupInfoParams),
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
