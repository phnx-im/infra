// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{ops::Deref, sync::Arc};

use async_trait::async_trait;
use phnxbackend::{
    messages::intra_backend::DsFanOutMessage,
    qs::{
        errors::QsEnqueueError, network_provider_trait::NetworkProvider,
        storage_provider_trait::QsStorageProvider, Fqdn, Qs, QsConnector, QsVerifyingKey,
    },
};

use crate::endpoints::qs::ws::DispatchWebsocketNotifier;

#[derive(Debug)]
pub struct MemoryEnqueueProvider<T: QsStorageProvider, N: NetworkProvider> {
    pub storage: Arc<T>,
    pub notifier: DispatchWebsocketNotifier,
    pub network: Arc<N>,
}

#[async_trait]
impl<T: QsStorageProvider, N: NetworkProvider> QsConnector for MemoryEnqueueProvider<T, N> {
    type EnqueueError = QsEnqueueError<T, N>;
    type VerifyingKeyError = T::LoadSigningKeyError;

    async fn dispatch(&self, message: DsFanOutMessage) -> Result<(), Self::EnqueueError> {
        Qs::enqueue_message(
            self.storage.deref(),
            &self.notifier,
            self.network.deref(),
            message,
        )
        .await
    }

    async fn verifying_key(&self, _fqdn: &Fqdn) -> Result<QsVerifyingKey, Self::VerifyingKeyError> {
        let key = self
            .storage
            .load_signing_key()
            .await?
            .verifying_key()
            .clone();
        Ok(key)
    }
}
