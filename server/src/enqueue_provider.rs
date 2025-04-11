// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxbackend::{
    messages::intra_backend::DsFanOutMessage,
    qs::{
        PushNotificationProvider, Qs, QsConnector, errors::QsEnqueueError,
        network_provider::NetworkProvider,
    },
};

use crate::endpoints::qs::ws::DispatchWebsocketNotifier;

#[derive(Debug, Clone)]
pub struct SimpleEnqueueProvider<N: NetworkProvider, P: PushNotificationProvider> {
    pub qs: Qs,
    pub notifier: DispatchWebsocketNotifier,
    pub push_notification_provider: P,
    pub network: N,
}

impl<N: NetworkProvider, P: PushNotificationProvider> QsConnector for SimpleEnqueueProvider<N, P> {
    type EnqueueError = QsEnqueueError<N>;

    fn dispatch(
        &self,
        message: DsFanOutMessage,
    ) -> impl Future<Output = Result<(), Self::EnqueueError>> + Send {
        Qs::enqueue_message(
            &self.qs,
            &self.notifier,
            &self.push_notification_provider,
            &self.network,
            message,
        )
    }
}
