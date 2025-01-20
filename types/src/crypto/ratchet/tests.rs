// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::LazyLock;

use rand::SeedableRng;

use crate::{
    codec::PhnxCodec,
    messages::{
        client_ds::{QsQueueMessagePayload, QsQueueMessageType},
        EncryptedQsQueueMessage,
    },
    time::TimeStamp,
};

use super::*;

// Test that the ratchet works.
#[test]
fn test_ratchet() {
    let ratchet_secret = RatchetSecret::random().unwrap();
    let mut sender_ratchet = QueueRatchet::try_from(ratchet_secret.clone()).unwrap();
    let mut receiver_ratchtet = QueueRatchet::try_from(ratchet_secret).unwrap();
    let message = QsQueueMessagePayload {
        timestamp: TimeStamp::now(),
        message_type: QsQueueMessageType::MlsMessage,
        payload: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
    };
    let encrypted_message = sender_ratchet.encrypt(message.clone()).unwrap();
    let plaintext: QsQueueMessagePayload = receiver_ratchtet.decrypt(encrypted_message).unwrap();
    assert_eq!(plaintext, message);
}

static RATCHET: LazyLock<QueueRatchet<EncryptedQsQueueMessage, QsQueueMessagePayload>> =
    LazyLock::new(|| {
        let mut rng = rand_chacha::ChaCha20Rng::from_seed([42; 32]);
        let secret = RatchetSecret::random_with_rng(&mut rng).unwrap();
        secret.try_into().unwrap()
    });

#[test]
fn ratchet_secret_serde_stability_json() {
    insta::assert_json_snapshot!(&*RATCHET);
}

#[test]
fn ratchet_secret_serde_stability_cbor() {
    let bytes = PhnxCodec::to_vec(&*RATCHET).unwrap();
    insta::assert_binary_snapshot!(".cbor", bytes);
}
