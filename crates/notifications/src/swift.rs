use std::ffi::c_void;

use tokio::sync::mpsc;
use tracing::error;

pub type DeliveredNotificationHandler = unsafe extern "C" fn(*const c_void, *const u8, u32);
pub type DeliveredNotificationFinisher = unsafe extern "C" fn(*mut c_void);

unsafe extern "C" {
    fn notifications_send(
        identifier_ptr: *const u8,
        identifier_len: u32,
        title_ptr: *const u8,
        title_len: u32,
        body_ptr: *const u8,
        body_len: u32,
    );

    fn notifications_remove(identifiers_ptr: *const u8, identifiers_len: u32);

    fn notifications_get_delivered(
        ctx: *mut c_void,
        handler: DeliveredNotificationHandler,
        finish: DeliveredNotificationFinisher,
    );
}

pub(crate) fn send_notification(identifier: &str, title: &str, body: &str) {
    unsafe {
        notifications_send(
            identifier.as_ptr(),
            identifier.len() as u32,
            title.as_ptr(),
            title.len() as u32,
            body.as_ptr(),
            body.len() as u32,
        );
    }
}

pub(crate) fn remove_notifications(identifiers: &[impl AsRef<str>]) {
    let mut buf = String::new();
    for identifier in identifiers {
        buf.push_str(identifier.as_ref());
        buf.push(0 as char);
    }
    unsafe {
        notifications_remove(buf.as_ptr(), (buf.len() as u32).saturating_sub(1));
    }
}

pub(crate) fn delivered_notifications() -> mpsc::UnboundedReceiver<String> {
    let (tx, rx) = mpsc::unbounded_channel();
    let tx = Box::into_raw(Box::new(tx));
    unsafe {
        notifications_get_delivered(tx as *mut _, send_identifier, drop_sender);
    }
    rx
}

unsafe extern "C" fn send_identifier(
    tx: *const c_void,
    identifier_ptr: *const u8,
    identifier_len: u32,
) {
    let tx = unsafe { &*(tx as *const mpsc::UnboundedSender<String>) };
    dbg!(identifier_len);
    let identifier = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(
            identifier_ptr,
            identifier_len as usize,
        ))
    };
    match identifier {
        Ok(identifier) => {
            let _ = tx.send(identifier.to_string());
        }
        Err(error) => error!(?error, "Failed to parse notification identifier"),
    }
}

unsafe extern "C" fn drop_sender(tx: *mut c_void) {
    let _ = unsafe { Box::from_raw(tx as *mut mpsc::UnboundedSender<String>) };
}
