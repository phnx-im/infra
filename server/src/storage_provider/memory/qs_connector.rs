// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{ops::Deref, sync::Arc};

use async_trait::async_trait;
use phnxbackend::{
    messages::intra_backend::DsFanOutMessage,
    qs::{
        errors::QsEnqueueError, network_provider_trait::NetworkProvider,
        storage_provider_trait::QsStorageProvider, PushNotificationProvider, Qs, QsConnector,
    },
};
use phnxtypes::{
    crypto::signatures::keys::QsVerifyingKey, errors::qs::QsVerifyingKeyError, identifiers::Fqdn,
};

use crate::endpoints::qs::ws::DispatchWebsocketNotifier;

#[derive(Debug)]
pub struct MemoryEnqueueProvider<
    T: QsStorageProvider,
    N: NetworkProvider,
    P: PushNotificationProvider,
> {
    pub storage: Arc<T>,
    pub notifier: DispatchWebsocketNotifier,
    pub push_token_provider: Arc<P>,
    pub network: N,
}

#[async_trait]
impl<T: QsStorageProvider, N: NetworkProvider, P: PushNotificationProvider> QsConnector
    for MemoryEnqueueProvider<T, N, P>
{
    type EnqueueError = QsEnqueueError<T, N>;
    type VerifyingKeyError = QsVerifyingKeyError;

    async fn dispatch(&self, message: DsFanOutMessage) -> Result<(), Self::EnqueueError> {
        Qs::enqueue_message(
            self.storage.deref(),
            &self.notifier,
            self.push_token_provider.deref(),
            &self.network,
            message,
        )
        .await
    }

    async fn verifying_key(&self, domain: Fqdn) -> Result<QsVerifyingKey, Self::VerifyingKeyError> {
        Qs::verifying_key(self.storage.deref(), &self.network, domain).await
    }
}
