// #[cfg(target_os = "android")]
mod kotlin;
#[cfg(any(target_os = "macos", target_os = "ios"))]
mod swift;

// #[cfg(target_os = "android")]
pub use kotlin::register_java_vm;

pub struct Notification {
    pub identifier: String,
    pub title: String,
    pub body: String,
}

#[allow(unused_variables)]
pub fn send(notification: Notification) {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    swift::send_notification(
        &notification.identifier,
        &notification.title,
        &notification.body,
    );
    #[cfg(target_os = "android")]
    {
        kotlin::send_notification(
            &notification.identifier,
            &notification.title,
            &notification.body,
        );
    }
    #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "android")))]
    {
        tracing::error!("send is not implemented for this platform");
    }
}

#[allow(unused_variables)]
pub fn remove(identifiers: &[impl AsRef<str>]) {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    swift::remove_notifications(identifiers);
    #[cfg(target_os = "android")]
    kotlin::remove_notifications(identifiers);
    #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "android")))]
    {
        tracing::error!("remove is not implemented for this platform");
    }
}

pub async fn delivered() -> Vec<String> {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        let mut rx = swift::delivered_notifications();
        let mut identifiers = Vec::new();
        while let Some(identifier) = rx.recv().await {
            identifiers.push(identifier);
        }
        identifiers
    }
    #[cfg(target_os = "android")]
    {
        kotlin::active_notifications()
    }
    #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "android")))]
    {
        tracing::error!("delived is not implemented for this platform");
        Vec::new()
    }
}
