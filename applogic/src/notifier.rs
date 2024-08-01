// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Result;
use notify_rust::Notification;
use tokio::sync::Mutex;

pub(crate) use phnxcoreclient::{ConversationId, ConversationMessage, NotificationType};

pub(crate) trait Notifiable
where
    Self: Clone,
{
    fn notify(&self, notification_type: NotificationType) -> bool;
    fn notifier(&self) -> Notifier<Self>
    where
        Self: Sized,
    {
        Notifier::new(self.clone())
    }
}

pub(crate) struct Notifier<T: Notifiable> {
    n: T,
}

impl<T: Notifiable> Notifier<T> {
    pub(crate) fn new(n: T) -> Self {
        Self { n }
    }

    pub(crate) fn notify(&self, notification_type: NotificationType) -> bool {
        self.n.notify(notification_type)
    }
}

pub(crate) struct NotificationHub<T: Notifiable> {
    pub(crate) sinks: Mutex<Vec<Notifier<T>>>,
}

impl<T: Notifiable> NotificationHub<T> {
    pub async fn add_sink(&self, sink: Notifier<T>) {
        let mut sinks = self.sinks.lock().await;
        sinks.push(sink);
    }

    /// Dispatch several notifications to the sinks.
    pub async fn dispatch_notifications(&self, notification_types: Vec<NotificationType>) {
        let mut sinks = self.sinks.lock().await;
        for notification_type in notification_types {
            sinks.retain(|sink| sink.notify(notification_type.clone()));
        }
    }
}

impl<T: Notifiable> Default for NotificationHub<T> {
    fn default() -> Self {
        Self {
            sinks: Mutex::new(vec![]),
        }
    }
}

/// Dispatch a notification to the flutter side if and only if a
/// notification hub is set.
pub(crate) async fn dispatch_conversation_notifications<T: Notifiable>(
    notification_hub: &NotificationHub<T>,
    conversation_ids: impl IntoIterator<Item = ConversationId>,
) {
    notification_hub
        .dispatch_notifications(
            conversation_ids
                .into_iter()
                .map(NotificationType::ConversationChange)
                .collect(),
        )
        .await;
}

/// Dispatch conversation message notifications to the flutter side if and
/// only if a notification hub is set.
pub(crate) async fn dispatch_message_notifications<T: Notifiable>(
    notification_hub: &NotificationHub<T>,
    conversation_messages: impl IntoIterator<Item = ConversationMessage>,
) {
    notification_hub
        .dispatch_notifications(
            conversation_messages
                .into_iter()
                .map(NotificationType::Message)
                .collect(),
        )
        .await;
}

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
pub(crate) fn init_desktop_os_notifications() -> Result<(), notify_rust::error::Error> {
    #[cfg(target_os = "macos")]
    {
        let res = notify_rust::set_application("im.phnx.prototype");
        if res.is_err() {
            log::warn!("Could not set application for desktop notifications");
        }
    }

    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
pub(crate) fn show_desktop_notifications(
    notifications: &[crate::api::notifications::NotificationContent],
) {
    for notification in notifications {
        if let Err(e) = Notification::new()
            .summary(notification.title.as_str())
            .body(notification.body.as_str())
            .show()
        {
            log::error!("Failed to send desktop notification: {}", e);
        }
    }
}
