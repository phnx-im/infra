use crate::types::*;

trait NotificationProvider {
    fn notify(&self, notification_type: NotificationType) -> bool;
}

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
    pub(crate) fn new(n: T) -> Self {
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
    pub(crate) fn dispatch_message_notification(
        &mut self,
        dispatched_conversation_message: DispatchedConversationMessage,
    ) {
        self.dispatch_notification(NotificationType::Message(dispatched_conversation_message));
    }

    pub(crate) fn dispatch_conversation_notification(&mut self) {
        self.dispatch_notification(NotificationType::ConversationChange);
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
