// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use async_trait::async_trait;
use phnxbackend::{
    messages::intra_backend::DsFanOutMessage,
    qs::{
        PushNotificationProvider, Qs, QsConnector, errors::QsEnqueueError,
        network_provider_trait::NetworkProvider,
    },
};

use crate::endpoints::qs::ws::DispatchWebsocketNotifier;

#[derive(Debug)]
pub struct SimpleEnqueueProvider<N: NetworkProvider, P: PushNotificationProvider> {
    pub qs: Qs,
    pub notifier: DispatchWebsocketNotifier,
    pub push_notification_provider: P,
    pub network: N,
}

#[async_trait]
impl<N: NetworkProvider, P: PushNotificationProvider> QsConnector for SimpleEnqueueProvider<N, P> {
    type EnqueueError = QsEnqueueError<N>;

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
}
