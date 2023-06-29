use crate::messages::client_ds::{QsQueueMessagePayload, QsQueueMessageType};

use super::*;

// Test that the ratchet works.
#[test]
fn test_ratchet() {
    let ratchet_secret = RatchetSecret::random().unwrap();
    let mut sender_ratchet = QueueRatchet::try_from(ratchet_secret.clone()).unwrap();
    let mut receiver_ratchtet = QueueRatchet::try_from(ratchet_secret).unwrap();
    let message = QsQueueMessagePayload {
        message_type: QsQueueMessageType::MlsMessage,
        payload: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
    };
    let encrypted_message = sender_ratchet.encrypt(message.clone()).unwrap();
    let plaintext: QsQueueMessagePayload = receiver_ratchtet.decrypt(encrypted_message).unwrap();
    assert_eq!(plaintext, message);
}
