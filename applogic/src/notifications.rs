// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use notify_rust::Notification;
use phnxcoreclient::{clients::CoreUser, ConversationId, ConversationMessage, NotificationType};

use anyhow::{anyhow, Result};

use crate::api::user::creation::User;

pub trait Notifiable
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

pub struct Notifier<T: Notifiable> {
    n: T,
}

impl<T: Notifiable> Notifier<T> {
    pub fn new(n: T) -> Self {
        Self { n }
    }

    pub(crate) fn notify(&self, notification_type: NotificationType) -> bool {
        self.n.notify(notification_type)
    }
}

pub(crate) struct NotificationHub<T: Notifiable> {
    pub(crate) sinks: Vec<Notifier<T>>,
}

impl<T: Notifiable> NotificationHub<T> {
    pub fn add_sink(&mut self, sink: Notifier<T>) {
        self.sinks.push(sink);
    }

    pub(crate) fn dispatch_message_notification(
        &mut self,
        conversation_message: ConversationMessage,
    ) {
        self.dispatch_notification(NotificationType::Message(conversation_message));
    }

    pub(crate) fn dispatch_conversation_notification(&mut self, conversation_id: ConversationId) {
        self.dispatch_notification(NotificationType::ConversationChange(conversation_id));
    }

    fn dispatch_notification(&mut self, notification_type: NotificationType) {
        self.sinks
            .retain(|sink| sink.notify(notification_type.clone()));
    }
}

impl<T: Notifiable> Default for NotificationHub<T> {
    fn default() -> Self {
        Self { sinks: vec![] }
    }
}

impl User {
    /// Dispatch a notification to the flutter side if and only if a
    /// notification hub is set.
    pub(crate) async fn dispatch_conversation_notifications(
        &self,
        conversation_ids: impl IntoIterator<Item = ConversationId>,
    ) {
        let mut notification_hub = self.notification_hub_option.lock().await;
        conversation_ids.into_iter().for_each(|conversation_id| {
            notification_hub.dispatch_conversation_notification(conversation_id)
        });
    }

    /// Dispatch conversation message notifications to the flutter side if and
    /// only if a notification hub is set.
    pub(crate) async fn dispatch_message_notifications(
        &self,
        conversation_messages: impl IntoIterator<Item = ConversationMessage>,
    ) {
        let mut notification_hub = self.notification_hub_option.lock().await;
        conversation_messages
            .into_iter()
            .for_each(|conversation_message| {
                notification_hub.dispatch_message_notification(conversation_message)
            });
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
    pub(crate) async fn send_desktop_os_message_notifications(
        &self,
        user: &CoreUser,
        conversation_messages: Vec<ConversationMessage>,
    ) -> Result<()> {
        let (summary, body) = match &conversation_messages[..] {
            [] => return Ok(()),
            [conversation_message] => {
                let conversation = user
                    .conversation(conversation_message.conversation_id())
                    .await
                    .ok_or(anyhow!("Conversation not found"))?;
                let summary = match conversation.conversation_type() {
                    phnxcoreclient::ConversationType::UnconfirmedConnection(username)
                    | phnxcoreclient::ConversationType::Connection(username) => {
                        username.to_string()
                    }
                    phnxcoreclient::ConversationType::Group => {
                        conversation.attributes().title().to_string()
                    }
                };
                let body = conversation_message
                    .message()
                    .string_representation(conversation.conversation_type());
                (summary, body)
            }
            _ => (
                "New messages".to_owned(),
                "You have received new messages.".to_owned(),
            ),
        };

        Notification::new()
            .summary(summary.as_str())
            .body(body.as_str())
            .show()?;

        Ok(())
    }

    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    pub(crate) async fn send_desktop_os_conversation_notifications(
        &self,
        user: &CoreUser,
        conversations: Vec<ConversationId>,
    ) -> Result<()> {
        let (summary, body) = match conversations[..] {
            [] => return Ok(()),
            [conversation] => {
                let conversation_title = user
                    .conversation(conversation)
                    .await
                    .ok_or(anyhow!("Conversation not found"))?
                    .attributes()
                    .title()
                    .to_string();
                let summary = "New conversation";
                let body = format!("You have been added to {}", conversation_title);
                (summary, body)
            }
            _ => {
                let summary = "New conversations";
                let body = "You have been added to new conversations.".to_owned();
                (summary, body)
            }
        };

        Notification::new()
            .summary(summary)
            .body(body.as_str())
            .show()?;

        Ok(())
    }

    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    pub(crate) async fn send_desktop_os_connection_notifications(
        &self,
        user: &CoreUser,
        connection_conversations: Vec<ConversationId>,
    ) -> Result<()> {
        let (summary, body) = match connection_conversations[..] {
            [] => return Ok(()),
            [conversation] => {
                let conversation = user
                    .conversation(conversation)
                    .await
                    .ok_or(anyhow!("Conversation not found"))?;
                let contact_name = match conversation.conversation_type() {
                    phnxcoreclient::ConversationType::UnconfirmedConnection(username)
                    | phnxcoreclient::ConversationType::Connection(username) => {
                        username.to_string()
                    }
                    phnxcoreclient::ConversationType::Group => {
                        return Err(anyhow!(
                            "Conversation is a regular group, not a connection."
                        ))
                    }
                };
                let summary = "New connection";
                let body = format!("{} has created a new connection with you.", contact_name);
                (summary, body)
            }
            _ => {
                let summary = "New connections";
                let body = "Multiple new connections have been created.".to_owned();
                (summary, body)
            }
        };

        Notification::new()
            .summary(summary)
            .body(body.as_str())
            .show()?;

        Ok(())
    }
}
