// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::openmls::prelude::KeyPackage;
use tls_codec::{Serialize, TlsSerialize, TlsSize};

use crate::{
    crypto::{
        ear::keys::KeyPackageEarKey,
        kdf::keys::RatchetSecret,
        signatures::{
            keys::{QsClientVerifyingKey, QsUserVerifyingKey},
            signable::{Signable, Signature, SignedStruct},
        },
        RatchetEncryptionKey,
    },
    identifiers::{QsClientId, QsUserId},
};

use super::{
    client_qs::{
        ClientKeyPackageParams, DeleteClientRecordParams, DeleteUserRecordParams,
        DequeueMessagesParams, KeyPackageParams, UpdateClientRecordParams, UpdateUserRecordParams,
    },
    push_token::EncryptedPushToken,
    FriendshipToken, MlsInfraVersion,
};

#[derive(Debug, TlsSerialize, TlsSize)]
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

    pub fn without_signature(payload: ClientToQsMessageTbsOut) -> Self {
        let signature = Signature::empty();
        Self { payload, signature }
    }
}

#[derive(Debug, TlsSerialize, TlsSize)]
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
        "ClientToQsMessage"
    }
}

impl SignedStruct<ClientToQsMessageTbsOut> for ClientToQsMessageOut {
    fn from_payload(payload: ClientToQsMessageTbsOut, signature: Signature) -> Self {
        Self { payload, signature }
    }
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct CreateUserRecordParamsOut {
    pub user_record_auth_key: QsUserVerifyingKey,
    pub friendship_token: FriendshipToken,
    pub client_record_auth_key: QsClientVerifyingKey,
    pub queue_encryption_key: RatchetEncryptionKey,
    pub encrypted_push_token: Option<EncryptedPushToken>,
    pub initial_ratchet_secret: RatchetSecret,
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct CreateClientRecordParamsOut {
    pub sender: QsUserId,
    pub client_record_auth_key: QsClientVerifyingKey,
    pub queue_encryption_key: RatchetEncryptionKey,
    pub encrypted_push_token: Option<EncryptedPushToken>,
    pub initial_ratchet_secret: RatchetSecret, // TODO: This can be dropped once we support PCS
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct PublishKeyPackagesParamsOut {
    pub sender: QsClientId,
    pub key_packages: Vec<KeyPackage>,
    pub friendship_ear_key: KeyPackageEarKey,
}

/// This enum contains variants for each DS endpoint.
#[derive(Debug, TlsSerialize, TlsSize)]
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
    KeyPackage(KeyPackageParams),
    // Messages
    DequeueMessages(DequeueMessagesParams),
    // Key material
    QsEncryptionKey,
}
