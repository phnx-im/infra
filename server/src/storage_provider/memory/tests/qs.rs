// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxbackend::qs::{
    client_record::QsClientRecord, storage_provider_trait::QsStorageProvider,
    user_record::QsUserRecord,
};
use phnxtypes::{
    crypto::{
        ear::Ciphertext,
        ratchet::QueueRatchet,
        signatures::keys::{QsClientSigningKey, QsUserSigningKey},
        RatchetDecryptionKey,
    },
    messages::{
        client_ds::QsQueueMessagePayload, EncryptedQsQueueMessage, FriendshipToken, QueueMessage,
    },
};

use crate::storage_provider::memory::qs::MemStorageProvider;

// Unit tests for MemStorageProvider
#[actix_rt::test]
async fn qs_mem_provider() {
    let provider = MemStorageProvider::new("example.com".try_into().unwrap());

    // Set up a user record
    let user_record = QsUserRecord::new(
        QsUserSigningKey::random().unwrap().verifying_key().clone(),
        FriendshipToken::random().unwrap(),
    );

    // Register user
    let user_id = provider.create_user(user_record.clone()).await.unwrap();

    // Test user loading
    let loaded_user_record = provider.load_user(&user_id).await.unwrap();
    assert_eq!(loaded_user_record, user_record);

    // Set up a client record
    let client_record = QsClientRecord::new(
        user_id,
        None,
        RatchetDecryptionKey::generate().unwrap().encryption_key(),
        QsClientSigningKey::random()
            .unwrap()
            .verifying_key()
            .clone(),
        QueueRatchet::<EncryptedQsQueueMessage, QsQueueMessagePayload>::random().unwrap(),
    );

    // Register client
    let client_id = provider.create_client(client_record.clone()).await.unwrap();

    // Test client loading
    let loaded_client_record = provider.load_client(&client_id).await.unwrap();
    assert_eq!(loaded_client_record, client_record);

    // Enqueue &dequeue a message

    let message = QueueMessage {
        sequence_number: 0,
        ciphertext: Ciphertext::default(),
    };

    provider.enqueue(&client_id, message.clone()).await.unwrap();

    let (messages, remaining_count) = provider.read_and_delete(&client_id, 0, 1).await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(remaining_count, 0);
    assert_eq!(messages[0], message);

    // Enqueue several messages & dequeue them in steps
    let mut messages = Vec::new();

    for i in 1..31 {
        let message = QueueMessage {
            sequence_number: i,
            ciphertext: Ciphertext::default(),
        };
        messages.push(message.clone());
        provider.enqueue(&client_id, message).await.unwrap();
    }

    // Test with a first batch
    let (first_batch, remaining_1) = provider.read_and_delete(&client_id, 1, 10).await.unwrap();
    assert_eq!(first_batch.len(), 10);
    assert_eq!(remaining_1, 20);

    // Test with a second batch
    let (second_batch, remaining_2) = provider.read_and_delete(&client_id, 11, 10).await.unwrap();
    assert_eq!(second_batch.len(), 10);
    assert_eq!(remaining_2, 10);

    // Test with a sequence number that is too low
    let (third_batch, remaining_3) = provider.read_and_delete(&client_id, 0, 5).await.unwrap();
    assert_eq!(third_batch.len(), 5);
    assert_eq!(remaining_3, 5);

    // Test with a maximum number of message sthat is too high
    let (fourth_batch, remaining_4) = provider.read_and_delete(&client_id, 26, 10).await.unwrap();
    assert_eq!(fourth_batch.len(), 5);
    assert_eq!(remaining_4, 0);
}
