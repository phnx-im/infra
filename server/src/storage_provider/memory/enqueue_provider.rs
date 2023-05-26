// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{ops::Deref, sync::Arc};

use async_trait::async_trait;
use phnxbackend::{
    messages::intra_backend::DsFanOutMessage,
    qs::{
        errors::QsEnqueueError, storage_provider_trait::QsStorageProvider, Fqdn, Qs,
        QsEnqueueProvider, QsVerifyingKey,
    },
};

use crate::endpoints::qs::ws::DispatchWebsocketNotifier;

#[derive(Debug)]
pub struct MemoryEnqueueProvider<T: QsStorageProvider> {
    pub storage: Arc<T>,
    pub notifier: DispatchWebsocketNotifier,
}

#[async_trait]
impl<T: QsStorageProvider> QsEnqueueProvider for MemoryEnqueueProvider<T> {
    type EnqueueError = QsEnqueueError<T>;
    type VerifyingKeyError = T::LoadSigningKeyError;

    async fn enqueue(&self, message: DsFanOutMessage) -> Result<(), Self::EnqueueError> {
        Qs::enqueue_message(self.storage.deref(), &self.notifier, message).await
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
