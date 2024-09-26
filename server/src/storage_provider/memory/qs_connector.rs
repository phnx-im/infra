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
    S: QsStorageProvider,
    N: NetworkProvider,
    P: PushNotificationProvider,
> {
    pub qs: Qs,
    pub storage: Arc<S>,
    pub notifier: DispatchWebsocketNotifier,
    pub push_notification_provider: P,
    pub network: N,
}

#[async_trait]
impl<S: QsStorageProvider, N: NetworkProvider, P: PushNotificationProvider> QsConnector
    for MemoryEnqueueProvider<S, N, P>
{
    type EnqueueError = QsEnqueueError<N>;
    type VerifyingKeyError = QsVerifyingKeyError;

    async fn dispatch(&self, message: DsFanOutMessage) -> Result<(), Self::EnqueueError> {
        Qs::enqueue_message(
            &self.qs,
            &self.notifier,
            &self.push_notification_provider,
            &self.network,
            message,
        )
        .await
    }

    async fn verifying_key(&self, domain: Fqdn) -> Result<QsVerifyingKey, Self::VerifyingKeyError> {
        self.qs.verifying_key(&self.network, domain).await
    }
}
