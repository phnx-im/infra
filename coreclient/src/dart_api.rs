// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::net::ToSocketAddrs;
use std::sync::Mutex;

use flutter_rust_bridge::{RustOpaque, StreamSink};

pub use crate::types::*;
use crate::{
    notifications::{Notifiable, NotificationHub},
    users::SelfUser,
};

#[path = "../dart-bridge/bridge_generated.rs"]
mod bridge_generated;

/// This is only to tell flutter_rust_bridge that it should expose the types
/// used in the parameters
pub fn _expose_rust_state(rust_state: RustState) -> RustState {
    rust_state
}

#[derive(Clone)]
pub struct DartNotifier {
    pub stream_sink: StreamSink<NotificationType>,
}

impl Notifiable for DartNotifier {
    fn notify(&self, notification_type: NotificationType) -> bool {
        self.stream_sink.add(notification_type)
    }
}

impl From<StreamSink<NotificationType>> for DartNotifier {
    fn from(stream_sink: StreamSink<NotificationType>) -> Self {
        Self { stream_sink }
    }
}

pub struct RustState {
    user: RustOpaque<Mutex<SelfUser<DartNotifier>>>,
}

impl RustState {
    #[tokio::main(flavor = "current_thread")]
    pub async fn new(
        user_name: String,
        password: String,
        address: String,
        stream_sink: StreamSink<NotificationType>,
    ) -> RustState {
        let dart_notifier = DartNotifier { stream_sink };
        let mut notification_hub = NotificationHub::<DartNotifier>::default();
        notification_hub.add_sink(dart_notifier.notifier());
        let user = SelfUser::new(
            &user_name,
            &password,
            address.to_socket_addrs().unwrap().next().unwrap(),
            notification_hub,
        )
        .await;
        Self {
            user: RustOpaque::new(Mutex::new(user)),
        }
    }

    #[tokio::main(flavor = "current_thread")]
    pub async fn create_connection(&self, user_name: String) {
        let mut user = self.user.lock().unwrap();
        user.add_contact(&user_name).await;
    }
}
