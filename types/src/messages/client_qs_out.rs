// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::openmls::prelude::KeyPackage;
use tls_codec::{Serialize, TlsSerialize, TlsSize, TlsVarInt};

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
        DequeueMessagesParams, KeyPackageBatchParams, UpdateClientRecordParams,
        UpdateUserRecordParams,
    },
    push_token::EncryptedPushToken,
    FriendshipToken,
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
    // This essentially includes the wire format.
    body: QsVersionedRequestParamsOut,
}

#[derive(Debug)]
enum QsVersionedRequestParamsOut {
    Alpha(QsRequestParamsOut),
}

impl QsVersionedRequestParamsOut {
    pub fn version(&self) -> TlsVarInt {
        match self {
            QsVersionedRequestParamsOut::Alpha(_) => TlsVarInt::new(1).expect("infallible"),
        }
    }
}

impl tls_codec::Size for QsVersionedRequestParamsOut {
    fn tls_serialized_len(&self) -> usize {
        match self {
            QsVersionedRequestParamsOut::Alpha(params) => {
                self.version().tls_serialized_len() + params.tls_serialized_len()
            }
        }
    }
}

// Note: Manual implementation because `TlsSerialize` does not support custom variant tags.
impl Serialize for QsVersionedRequestParamsOut {
    fn tls_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, tls_codec::Error> {
        match self {
            QsVersionedRequestParamsOut::Alpha(params) => {
                Ok(self.version().tls_serialize(writer)? + params.tls_serialize(writer)?)
            }
        }
    }
}

impl ClientToQsMessageTbsOut {
    pub fn new(body: QsRequestParamsOut) -> Self {
        Self {
            body: QsVersionedRequestParamsOut::Alpha(body),
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
    KeyPackageBatch(KeyPackageBatchParams),
    // Messages
    DequeueMessages(DequeueMessagesParams),
    // Key material
    QsVerifyingKey,
    QsEncryptionKey,
}

#[cfg(test)]
mod tests {
    use tls_codec::DeserializeBytes;
    use uuid::Uuid;

    use crate::{
        crypto::{ear::Ciphertext, signatures::private_keys::VerifyingKey},
        messages::client_qs::{
            QsRequestParams, QsVersionedRequestParams, VerifiableClientToQsMessage,
        },
    };

    use super::*;

    #[test]
    fn create_user_api_stability() {
        let token = FriendshipToken::new_for_test(b"friendship_token".to_vec());
        let create_user_record_params = CreateUserRecordParamsOut {
            user_record_auth_key: QsUserVerifyingKey::new_for_test(VerifyingKey::new_for_test(
                b"user_record_auth_key".to_vec(),
            )),
            friendship_token: token.clone(),
            client_record_auth_key: QsClientVerifyingKey::new_for_test(VerifyingKey::new_for_test(
                b"client_record_auth_key".to_vec(),
            )),
            queue_encryption_key: RatchetEncryptionKey::new_for_test(
                b"encryption_key".to_vec().into(),
            ),
            encrypted_push_token: Some(EncryptedPushToken::from(Ciphertext::dummy())),
            initial_ratchet_secret: RatchetSecret::new_for_test(
                (*b"_initial_ratchet_secret_32_bytes").into(),
            ),
        };
        let message_out = ClientToQsMessageOut {
            payload: ClientToQsMessageTbsOut {
                body: QsVersionedRequestParamsOut::Alpha(QsRequestParamsOut::CreateUser(
                    create_user_record_params,
                )),
            },
            signature: Signature::from_token(token.clone()),
        };

        let message_out_tls = message_out.tls_serialize_detached().unwrap();
        match VerifiableClientToQsMessage::tls_deserialize_exact_bytes(&message_out_tls)
            .unwrap()
            .verify_with_token(token)
            .unwrap()
        {
            QsVersionedRequestParams::Alpha(QsRequestParams::CreateUser(_)) => {}
            _ => panic!("expected CreateUser variant"),
        }

        insta::assert_binary_snapshot!(".tls", message_out_tls);
    }

    #[test]
    fn update_user_api_stability() {
        let token = FriendshipToken::new_for_test(b"friendship_token".to_vec());
        let update_user_record_params = UpdateUserRecordParams {
            sender: QsUserId::from(Uuid::from_u128(1)),
            user_record_auth_key: QsUserVerifyingKey::new_for_test(VerifyingKey::new_for_test(
                b"user_record_auth_key".to_vec(),
            )),
            friendship_token: token.clone(),
        };
        let message_out = ClientToQsMessageOut {
            payload: ClientToQsMessageTbsOut {
                body: QsVersionedRequestParamsOut::Alpha(QsRequestParamsOut::UpdateUser(
                    update_user_record_params,
                )),
            },
            signature: Signature::from_token(token.clone()),
        };

        let message_out_tls = message_out.tls_serialize_detached().unwrap();
        match VerifiableClientToQsMessage::tls_deserialize_exact_bytes(&message_out_tls)
            .unwrap()
            .verify_with_token(token)
            .unwrap()
        {
            QsVersionedRequestParams::Alpha(QsRequestParams::UpdateUser(_)) => {}
            _ => panic!("expected UpdateUser variant"),
        }

        insta::assert_binary_snapshot!(".tls", message_out_tls);
    }

    #[test]
    fn delete_user_api_stability() {
        let token = FriendshipToken::new_for_test(b"friendship_token".to_vec());
        let delete_user_record_params = DeleteUserRecordParams {
            sender: QsUserId::from(Uuid::from_u128(1)),
        };
        let message_out = ClientToQsMessageOut {
            payload: ClientToQsMessageTbsOut {
                body: QsVersionedRequestParamsOut::Alpha(QsRequestParamsOut::DeleteUser(
                    delete_user_record_params,
                )),
            },
            signature: Signature::from_token(token.clone()),
        };

        let message_out_tls = message_out.tls_serialize_detached().unwrap();
        match VerifiableClientToQsMessage::tls_deserialize_exact_bytes(&message_out_tls)
            .unwrap()
            .verify_with_token(token)
            .unwrap()
        {
            QsVersionedRequestParams::Alpha(QsRequestParams::DeleteUser(_)) => {}
            _ => panic!("expected DeleteUser variant"),
        }

        insta::assert_binary_snapshot!(".tls", message_out_tls);
    }

    #[test]
    fn create_client_api_stability() {
        let token = FriendshipToken::new_for_test(b"friendship_token".to_vec());
        let create_client_record_params = CreateClientRecordParamsOut {
            sender: QsUserId::from(Uuid::from_u128(1)),
            client_record_auth_key: QsClientVerifyingKey::new_for_test(VerifyingKey::new_for_test(
                b"client_record_auth_key".to_vec(),
            )),
            queue_encryption_key: RatchetEncryptionKey::new_for_test(
                b"encryption_key".to_vec().into(),
            ),
            encrypted_push_token: Some(EncryptedPushToken::from(Ciphertext::dummy())),
            initial_ratchet_secret: RatchetSecret::new_for_test(
                (*b"_initial_ratchet_secret_32_bytes").into(),
            ),
        };
        let message_out = ClientToQsMessageOut {
            payload: ClientToQsMessageTbsOut {
                body: QsVersionedRequestParamsOut::Alpha(QsRequestParamsOut::CreateClient(
                    create_client_record_params,
                )),
            },
            signature: Signature::from_token(token.clone()),
        };

        let message_out_tls = message_out.tls_serialize_detached().unwrap();
        match VerifiableClientToQsMessage::tls_deserialize_exact_bytes(&message_out_tls)
            .unwrap()
            .verify_with_token(token)
            .unwrap()
        {
            QsVersionedRequestParams::Alpha(QsRequestParams::CreateClient(_)) => {}
            _ => panic!("expected CreateClient variant"),
        }

        insta::assert_binary_snapshot!(".tls", message_out_tls);
    }

    #[test]
    fn update_client_api_stability() {
        let token = FriendshipToken::new_for_test(b"friendship_token".to_vec());
        let update_client_record_params = UpdateClientRecordParams {
            sender: QsClientId::from(Uuid::from_u128(1)),
            client_record_auth_key: QsClientVerifyingKey::new_for_test(VerifyingKey::new_for_test(
                b"client_record_auth_key".to_vec(),
            )),
            queue_encryption_key: RatchetEncryptionKey::new_for_test(
                b"encryption_key".to_vec().into(),
            ),
            encrypted_push_token: Some(EncryptedPushToken::from(Ciphertext::dummy())),
        };
        let message_out = ClientToQsMessageOut {
            payload: ClientToQsMessageTbsOut {
                body: QsVersionedRequestParamsOut::Alpha(QsRequestParamsOut::UpdateClient(
                    update_client_record_params,
                )),
            },
            signature: Signature::from_token(token.clone()),
        };

        let message_out_tls = message_out.tls_serialize_detached().unwrap();
        match VerifiableClientToQsMessage::tls_deserialize_exact_bytes(&message_out_tls)
            .unwrap()
            .verify_with_token(token)
            .unwrap()
        {
            QsVersionedRequestParams::Alpha(QsRequestParams::UpdateClient(_)) => {}
            _ => panic!("expected UpdateClient variant"),
        }

        insta::assert_binary_snapshot!(".tls", message_out_tls);
    }

    #[test]
    fn delete_client_api_stability() {
        let token = FriendshipToken::new_for_test(b"friendship_token".to_vec());
        let delete_client_record_params = DeleteClientRecordParams {
            sender: QsClientId::from(Uuid::from_u128(1)),
        };
        let message_out = ClientToQsMessageOut {
            payload: ClientToQsMessageTbsOut {
                body: QsVersionedRequestParamsOut::Alpha(QsRequestParamsOut::DeleteClient(
                    delete_client_record_params,
                )),
            },
            signature: Signature::from_token(token.clone()),
        };

        let message_out_tls = message_out.tls_serialize_detached().unwrap();
        match VerifiableClientToQsMessage::tls_deserialize_exact_bytes(&message_out_tls)
            .unwrap()
            .verify_with_token(token)
            .unwrap()
        {
            QsVersionedRequestParams::Alpha(QsRequestParams::DeleteClient(_)) => {}
            _ => panic!("expected DeleteClient variant"),
        }

        insta::assert_binary_snapshot!(".tls", message_out_tls);
    }

    #[test]
    fn publish_key_packages_api_stability() {
        let token = FriendshipToken::new_for_test(b"friendship_token".to_vec());
        let publish_key_packages_params = PublishKeyPackagesParamsOut {
            sender: QsClientId::from(Uuid::from_u128(1)),
            key_packages: vec![], // Note: No easy way to create a key package for testing.
            friendship_ear_key: KeyPackageEarKey::new_for_test(
                (*b"friendship_ear_key_32_bytes__pad").into(),
            ),
        };
        let message_out = ClientToQsMessageOut {
            payload: ClientToQsMessageTbsOut {
                body: QsVersionedRequestParamsOut::Alpha(QsRequestParamsOut::PublishKeyPackages(
                    publish_key_packages_params,
                )),
            },
            signature: Signature::from_token(token.clone()),
        };
        let message_out_tls = message_out.tls_serialize_detached().unwrap();

        let message_out =
            VerifiableClientToQsMessage::tls_deserialize_exact_bytes(&message_out_tls).unwrap();
        let message_out = message_out.verify_with_token(token).unwrap();

        match message_out {
            QsVersionedRequestParams::Alpha(QsRequestParams::PublishKeyPackages(_)) => {}
            _ => panic!("expected PublishKeyPackages variant"),
        }

        insta::assert_binary_snapshot!(".tls", message_out_tls);
    }

    #[test]
    fn client_key_package_api_stability() {
        let token = FriendshipToken::new_for_test(b"friendship_token".to_vec());
        let client_key_package_params = ClientKeyPackageParams {
            sender: QsUserId::from(Uuid::from_u128(1)),
            client_id: QsClientId::from(Uuid::from_u128(2)),
        };
        let message_out = ClientToQsMessageOut {
            payload: ClientToQsMessageTbsOut {
                body: QsVersionedRequestParamsOut::Alpha(QsRequestParamsOut::ClientKeyPackage(
                    client_key_package_params,
                )),
            },
            signature: Signature::from_token(token.clone()),
        };

        let message_out_tls = message_out.tls_serialize_detached().unwrap();
        match VerifiableClientToQsMessage::tls_deserialize_exact_bytes(&message_out_tls)
            .unwrap()
            .verify_with_token(token)
            .unwrap()
        {
            QsVersionedRequestParams::Alpha(QsRequestParams::ClientKeyPackage(_)) => {}
            _ => panic!("expected ClientKeyPackage variant"),
        }

        insta::assert_binary_snapshot!(".tls", message_out_tls);
    }

    #[test]
    fn key_package_batch_api_stability() {
        let token = FriendshipToken::new_for_test(b"friendship_token".to_vec());
        let key_package_batch_params = KeyPackageBatchParams {
            sender: FriendshipToken::new_for_test(b"friendship_token".to_vec()),
            friendship_ear_key: KeyPackageEarKey::new_for_test(
                (*b"friendship_ear_key_32_bytes__pad").into(),
            ),
        };
        let message_out = ClientToQsMessageOut {
            payload: ClientToQsMessageTbsOut {
                body: QsVersionedRequestParamsOut::Alpha(QsRequestParamsOut::KeyPackageBatch(
                    key_package_batch_params,
                )),
            },
            signature: Signature::from_token(token.clone()),
        };

        let message_out_tls = message_out.tls_serialize_detached().unwrap();
        match VerifiableClientToQsMessage::tls_deserialize_exact_bytes(&message_out_tls)
            .unwrap()
            .verify_with_token(token)
            .unwrap()
        {
            QsVersionedRequestParams::Alpha(QsRequestParams::KeyPackageBatch(_)) => {}
            _ => panic!("expected KeyPackageBatch variant"),
        }

        insta::assert_binary_snapshot!(".tls", message_out_tls);
    }

    #[test]
    fn dequeue_messages_api_stability() {
        let token = FriendshipToken::new_for_test(b"friendship_token".to_vec());
        let dequeue_messages_params = DequeueMessagesParams {
            sender: QsClientId::from(Uuid::from_u128(1)),
            sequence_number_start: 1,
            max_message_number: 42,
        };
        let message_out = ClientToQsMessageOut {
            payload: ClientToQsMessageTbsOut {
                body: QsVersionedRequestParamsOut::Alpha(QsRequestParamsOut::DequeueMessages(
                    dequeue_messages_params,
                )),
            },
            signature: Signature::from_token(token.clone()),
        };

        let message_out_tls = message_out.tls_serialize_detached().unwrap();
        match VerifiableClientToQsMessage::tls_deserialize_exact_bytes(&message_out_tls)
            .unwrap()
            .verify_with_token(token)
            .unwrap()
        {
            QsVersionedRequestParams::Alpha(QsRequestParams::DequeueMessages(_)) => {}
            _ => panic!("expected DequeueMessages variant"),
        }

        insta::assert_binary_snapshot!(".tls", message_out_tls);
    }

    #[test]
    fn qs_verifying_key_api_stability() {
        let message_out = ClientToQsMessageOut {
            payload: ClientToQsMessageTbsOut {
                body: QsVersionedRequestParamsOut::Alpha(QsRequestParamsOut::QsVerifyingKey),
            },
            signature: Signature::empty(),
        };

        let message_out_tls = message_out.tls_serialize_detached().unwrap();
        match VerifiableClientToQsMessage::tls_deserialize_exact_bytes(&message_out_tls)
            .unwrap()
            .extract_without_verification()
            .unwrap()
        {
            QsVersionedRequestParams::Alpha(QsRequestParams::VerifyingKey) => {}
            _ => panic!("expected VerifyingKey variant"),
        }

        insta::assert_binary_snapshot!(".tls", message_out_tls);
    }

    #[test]
    fn qs_encryption_key_api_stability() {
        let message_out = ClientToQsMessageOut {
            payload: ClientToQsMessageTbsOut {
                body: QsVersionedRequestParamsOut::Alpha(QsRequestParamsOut::QsEncryptionKey),
            },
            signature: Signature::empty(),
        };

        let message_out_tls = message_out.tls_serialize_detached().unwrap();
        match VerifiableClientToQsMessage::tls_deserialize_exact_bytes(&message_out_tls)
            .unwrap()
            .extract_without_verification()
            .unwrap()
        {
            QsVersionedRequestParams::Alpha(QsRequestParams::EncryptionKey) => {}
            _ => panic!("expected EncryptionKey variant"),
        }

        insta::assert_binary_snapshot!(".tls", message_out_tls);
    }
}
