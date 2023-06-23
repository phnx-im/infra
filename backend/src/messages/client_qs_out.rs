// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use tls_codec::{Serialize, TlsSerialize, TlsSize};

use crate::{
    crypto::{
        ear::keys::AddPackageEarKey,
        kdf::keys::RatchetSecret,
        signatures::{
            keys::{QsClientVerifyingKey, QsUserVerifyingKey},
            signable::{Signable, Signature, SignedStruct},
        },
        RatchetEncryptionKey,
    },
    qs::{AddPackage, EncryptedPushToken, QsClientId, QsUserId},
};

use super::{
    client_qs::{
        ClientKeyPackageParams, DeleteClientRecordParams, DeleteUserRecordParams,
        DequeueMessagesParams, KeyPackageBatchParams, UpdateClientRecordParams,
        UpdateUserRecordParams,
    },
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
        "ClientToQsMessageTbs"
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
    pub add_packages: Vec<AddPackage>,
    pub add_package_ear_key: AddPackageEarKey,
    pub encrypted_push_token: Option<EncryptedPushToken>,
    pub initial_ratchet_secret: RatchetSecret,
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct CreateClientRecordParamsOut {
    pub sender: QsUserId,
    pub client_record_auth_key: QsClientVerifyingKey,
    pub queue_encryption_key: RatchetEncryptionKey,
    pub add_packages: Vec<AddPackage>,
    pub friendship_ear_key: AddPackageEarKey,
    pub encrypted_push_token: Option<EncryptedPushToken>,
    pub initial_ratchet_secret: RatchetSecret, // TODO: This can be dropped once we support PCS
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct PublishKeyPackagesParamsOut {
    pub sender: QsClientId,
    pub add_packages: Vec<AddPackage>,
    pub friendship_ear_key: AddPackageEarKey,
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
    KeyPackageBatch(KeyPackageBatchParams),
    // Messages
    DequeueMessages(DequeueMessagesParams),
    // Key material
    QsVerifyingKey,
    QsEncryptionKey,
}
