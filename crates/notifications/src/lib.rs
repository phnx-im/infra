#[cfg(any(target_os = "macos", target_os = "ios"))]
mod swift;

pub struct Notification {
    pub identifier: String,
    pub title: String,
    pub body: String,
}

pub fn send(notification: Notification) {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    swift::send_notification(
        &notification.identifier,
        &notification.title,
        &notification.body,
    );
}

pub fn remove(identifiers: &[impl AsRef<str>]) {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    swift::remove_notifications(identifiers);
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
    #[cfg(any(target_os = "android", target_os = "linux", target_os = "windows"))]
    {
        Vec::new()
    }
}
