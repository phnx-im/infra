use std::{error::Error, fmt::Debug};

use async_trait::async_trait;

use crate::messages::client_qs::EnqueuedMessage;

use super::{fanout_queue::FanOutQueueInfo, QueueId};

/// Storage provider trait for the QS.
#[async_trait]
pub trait QsStorageProvider: Sync + Send + Debug + 'static {
    type LoadQueueInfoError: Error + Debug + PartialEq + Eq + Clone;
    type SaveQueueInfoError: Error + Debug + PartialEq + Eq + Clone;
    type CreateQueueError: Error + Debug + PartialEq + Eq + Clone;
    type DeleteQueueError: Error + Debug + PartialEq + Eq + Clone;
    type EnqueueError: Error + Debug + PartialEq + Eq + Clone;
    type ReadAndDeleteError: Error + Debug + PartialEq + Eq + Clone;

    /// Load the info for the queue with the given queue id.
    async fn load_queue_info(&self, queue_id: &QueueId) -> Option<FanOutQueueInfo>;

    /// Saves info of the queue with the given id.
    async fn save_queue_info(
        &self,
        queue_id: &QueueId,
        queue_info: FanOutQueueInfo,
    ) -> Result<(), Self::SaveQueueInfoError>;

    /// Creates a fresh, initially emtpy queue with the given info. Returns
    /// an error if the queue id is already taken.
    /// TODO: It's not clear if the queue id should be sampled by the storage
    /// provider or the backend.
    async fn create_queue(
        &self,
        queue_id: &QueueId,
        queue_info: FanOutQueueInfo,
    ) -> Result<(), Self::CreateQueueError>;

    /// Deletes the queue with the given id, as well as the associated info.
    async fn delete_queue(&self, queue_id: &QueueId) -> Result<(), Self::DeleteQueueError>;

    /// Append the given message to the queue. Returns an error if the payload
    /// is greater than the maximum payload allowed by the storage provider.
    async fn enqueue(
        &self,
        queue_id: &QueueId,
        message: EnqueuedMessage,
    ) -> Result<(), Self::EnqueueError>;

    /// Delete all messages older than the given sequence number in the queue
    /// with the given id and return up to the requested number of messages from
    /// the queue starting with the message with the given sequence number, as
    /// well as the number of unread messages remaining in the queue.
    async fn read_and_delete(
        &self,
        queue_id: &QueueId,
        sequence_number: u64,
        number_of_messages: u64,
    ) -> Result<(Vec<EnqueuedMessage>, u64), Self::ReadAndDeleteError>;
}
