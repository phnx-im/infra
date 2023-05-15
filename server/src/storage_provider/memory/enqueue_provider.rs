use async_trait::async_trait;
use phnxbackend::{
    messages::intra_backend::DsFanOutMessage,
    qs::{
        errors::QsEnqueueError, storage_provider_trait::QsStorageProvider, Fqdn, Qs,
        QsEnqueueProvider, QsVerifyingKey,
    },
};

use crate::endpoints::qs::ws::DispatchWebsocketNotifier;

use super::qs::{LoadSigningKeyError, MemStorageProvider};

#[derive(Debug)]
pub struct MemoryEnqueueProvider<'a> {
    storage: &'a MemStorageProvider,
    notifier: &'a DispatchWebsocketNotifier,
}

#[async_trait]
impl<'a> QsEnqueueProvider for MemoryEnqueueProvider<'a> {
    type EnqueueError = QsEnqueueError<MemStorageProvider>;
    type VerifyingKeyError = LoadSigningKeyError;

    async fn enqueue(&self, message: DsFanOutMessage) -> Result<(), Self::EnqueueError> {
        Qs::enqueue_message(self.storage, self.notifier, message).await
    }

    async fn verifying_key(&self, fqdn: &Fqdn) -> Result<QsVerifyingKey, Self::VerifyingKeyError> {
        let key = self
            .storage
            .load_signing_key()
            .await?
            .verifying_key()
            .clone();
        Ok(key)
    }
}
