// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use airbackend::{
    messages::intra_backend::DsFanOutMessage,
    qs::{
        PushNotificationProvider, Qs, QsConnector, errors::QsEnqueueError,
        network_provider::NetworkProvider,
    },
};

#[derive(Debug, Clone)]
pub struct SimpleEnqueueProvider<N: NetworkProvider, P: PushNotificationProvider> {
    pub qs: Qs,
    pub push_notification_provider: P,
    pub network: N,
}

impl<N: NetworkProvider, P: PushNotificationProvider> QsConnector for SimpleEnqueueProvider<N, P> {
    type EnqueueError = QsEnqueueError<N>;

    fn dispatch(
        &self,
        message: DsFanOutMessage,
    ) -> impl Future<Output = Result<(), Self::EnqueueError>> + Send {
        self.qs
            .enqueue_message(&self.push_notification_provider, &self.network, message)
    }
}
