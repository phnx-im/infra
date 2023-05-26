// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use tls_codec::{Serialize, TlsSerialize, TlsSize};

use crate::{
    crypto::{
        ear::keys::FriendshipEarKey,
        signatures::{
            keys::{QsClientVerifyingKey, QsUserVerifyingKey},
            signable::{Signable, Signature, SignedStruct},
        },
        QueueRatchet, RatchetPublicKey,
    },
    qs::{AddPackage, EncryptedPushToken, QsClientId, QsUserId},
};

use super::{
    client_qs::{
        ClientKeyPackageParams, CreateClientRecordParams, DeleteClientRecordParams,
        DeleteUserRecordParams, DequeueMessagesParams, KeyPackageBatchParams,
        PublishKeyPackagesParams, UpdateClientRecordParams, UpdateUserRecordParams,
    },
    FriendshipToken, MlsInfraVersion,
};

#[derive(TlsSerialize, TlsSize)]
pub struct ClientToQsMessageOut {
    payload: ClientToQsMessageTbsOut,
    // Signature over all of the above.
    signature: Signature,
}

impl ClientToQsMessageOut {
    pub fn from_token(payload: ClientToQsMessageTbsOut, token: FriendshipToken) -> Self {
        let signature = Signature::from_token(token);
        Self { payload, signature }
    }
}

#[derive(TlsSerialize, TlsSize)]
pub struct ClientToQsMessageTbsOut {
    version: MlsInfraVersion,
    // This essentially includes the wire format.
    body: QsRequestParamsOut,
}

impl ClientToQsMessageTbsOut {
    pub fn new(body: QsRequestParamsOut) -> Self {
        Self {
            version: MlsInfraVersion::default(),
            body,
        }
    }
}

impl Signable for ClientToQsMessageTbsOut {
    type SignedOutput = ClientToQsMessageOut;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        "ClientToQsMessageTbs"
    }
}

impl SignedStruct<ClientToQsMessageTbsOut> for ClientToQsMessageOut {
    fn from_payload(payload: ClientToQsMessageTbsOut, signature: Signature) -> Self {
        Self { payload, signature }
    }
}

#[derive(TlsSerialize, TlsSize)]
pub struct CreateUserRecordParamsOut {
    pub user_record_auth_key: QsUserVerifyingKey,
    pub friendship_token: FriendshipToken,
    pub client_record_auth_key: QsClientVerifyingKey,
    pub queue_encryption_key: RatchetPublicKey,
    pub add_packages: Vec<AddPackage>,
    pub friendship_ear_key: FriendshipEarKey,
    pub encrypted_push_token: Option<EncryptedPushToken>,
    pub initial_ratchet_key: QueueRatchet,
}

#[derive(TlsSerialize, TlsSize)]
pub struct CreateClientRecordParamsOut {
    pub sender: QsUserId,
    pub client_record_auth_key: QsClientVerifyingKey,
    pub queue_encryption_key: RatchetPublicKey,
    pub add_packages: Vec<AddPackage>,
    pub friendship_ear_key: FriendshipEarKey,
    pub encrypted_push_token: Option<EncryptedPushToken>,
    pub initial_ratchet_key: QueueRatchet, // TODO: This can be dropped once we support PCS
}

#[derive(TlsSerialize, TlsSize)]
pub struct PublishKeyPackagesParamsOut {
    pub sender: QsClientId,
    pub add_packages: Vec<AddPackage>,
    pub friendship_ear_key: FriendshipEarKey,
}

/// This enum contains variants for each DS endpoint.
#[derive(TlsSerialize, TlsSize)]
#[repr(u8)]
pub enum QsRequestParamsOut {
    // User
    CreateUser(CreateUserRecordParamsOut),
    UpdateUser(UpdateUserRecordParams),
    DeleteUser(DeleteUserRecordParams),
    // Client
    CreateClient(CreateClientRecordParamsOut),
    UpdateClient(UpdateClientRecordParams),
    DeleteClient(DeleteClientRecordParams),
    // Key packages
    PublishKeyPackages(PublishKeyPackagesParamsOut),
    ClientKeyPackage(ClientKeyPackageParams),
    KeyPackageBatch(KeyPackageBatchParams),
    // Messages
    DequeueMessages(DequeueMessagesParams),
}
