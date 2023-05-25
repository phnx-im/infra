use crate::types::NotificationType;

pub(crate) trait CoreClientProvider: Send + Sync {
    type NotificationProvider: NotificationProvider;
    fn notification_provider(&self) -> &Self::NotificationProvider;
}

pub(crate) trait NotificationProvider {
    fn notify(&self, notification_type: NotificationType) -> bool;
}
