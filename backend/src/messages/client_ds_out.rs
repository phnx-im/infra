// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module contains structs and enums that represent messages that are
//! passed between clients and the backend.
//! TODO: We should eventually factor this module out, together with the crypto
//! module, to allow re-use by the client implementation.

use mls_assist::{
    messages::AssistedWelcome, treesync::RatchetTree, Extensions, GroupId, GroupInfo,
    MlsMessageOut, RatchetTreeIn, Signature as MlsAssistSignature, VerifiableGroupInfo,
};
use thiserror::Error;
use tls_codec::{Serialize, TlsDeserialize, TlsSerialize, TlsSize};

use crate::{
    crypto::{
        self,
        ear::keys::GroupStateEarKey,
        signatures::{
            keys::UserAuthKey,
            signable::{Signable, Signature, SignedStruct},
        },
    },
    ds::{
        errors::DsProcessingError,
        group_state::{EncryptedCredentialChain, UserKeyHash},
    },
    qs::{KeyPackageBatch, QsClientReference, VERIFIED},
};

use super::MlsInfraVersion;

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
    pub encrypted_credential_chain: EncryptedCredentialChain,
    pub creator_client_reference: QsClientReference,
    pub creator_user_auth_key: UserAuthKey,
    pub group_info: GroupInfo,
}

#[derive(TlsSerialize, TlsSize)]
pub struct AddUsersParamsOut {
    // The commit and a partial assisted group info.
    pub commit: (MlsMessageOut, (MlsAssistSignature, Extensions)),
    pub sender: UserKeyHash,
    pub welcome: AssistedWelcome,
    pub encrypted_welcome_attribution_infos: Vec<Vec<u8>>,
    pub key_package_batches: Vec<KeyPackageBatch<VERIFIED>>,
}

#[derive(TlsSerialize, TlsSize)]
#[repr(u8)]
pub enum DsRequestParamsOut {
    AddUsers(AddUsersParamsOut),
    CreateGroupParams(CreateGroupParamsOut),
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
