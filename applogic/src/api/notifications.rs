use std::sync::Arc;

use flutter_rust_bridge::{DartFnFuture, frb};

#[derive(Debug)]
pub struct NotificationContent {
    pub identifier: String,
    pub title: String,
    pub body: String,
    pub conversation_id: Option<String>,
}

#[derive(Debug)]
pub struct NotificationHandle {
    pub identifier: String,
    pub conversation_id: Option<String>,
}

#[frb(opaque)]
#[derive(Clone)]
pub struct NotificationService {
    callback: Arc<Callbacks>,
}

#[frb(ignore)]
struct Callbacks {
    send: Box<dyn Fn(NotificationContent) -> DartFnFuture<()> + Send + Sync + 'static>,
    get_active: Box<dyn Fn() -> DartFnFuture<Vec<NotificationHandle>> + Send + Sync + 'static>,
    remove: Box<dyn Fn(Vec<String>) -> DartFnFuture<()> + Send + Sync + 'static>,
}

impl NotificationService {
    #[frb(sync)]
    pub fn new(
        send: impl Fn(NotificationContent) -> DartFnFuture<()> + Send + Sync + 'static,
        get_active: impl Fn() -> DartFnFuture<Vec<NotificationHandle>> + Send + Sync + 'static,
        remove: impl Fn(Vec<String>) -> DartFnFuture<()> + Send + Sync + 'static,
    ) -> NotificationService {
        NotificationService {
            callback: Arc::new(Callbacks {
                send: Box::new(send),
                get_active: Box::new(get_active),
                remove: Box::new(remove),
            }),
        }
    }

    pub(crate) async fn send_notification(&self, notification: NotificationContent) {
        (self.callback.send)(notification).await;
    }

    pub(crate) async fn get_active_notifications(&self) -> Vec<NotificationHandle> {
        (self.callback.get_active)().await
    }

    pub(crate) async fn remove_notifications(&self, identifiers: Vec<String>) {
        (self.callback.remove)(identifiers).await;
    }
}
