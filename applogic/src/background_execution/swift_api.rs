use std::ffi::{c_char, CStr, CString};

use crate::{
    api::mobile_logging::init_logger,
    background_execution::{processing::retrieve_messages_sync, IncomingNotificationContent},
};

/// This method gets called from the iOS NSE
///
/// # Safety
///
/// The caller must ensure that the content is a pointer to a valid C string.
#[no_mangle]
pub unsafe extern "C" fn process_new_messages(content: *const c_char) -> *mut c_char {
    assert!(!content.is_null());

    let c_str = unsafe { CStr::from_ptr(content) };

    init_logger();

    let json_str = c_str.to_str().unwrap();
    let incoming_content: IncomingNotificationContent = serde_json::from_str(json_str).unwrap();

    // Retrieve messages
    let batch = retrieve_messages_sync(incoming_content.path);

    let response = serde_json::to_string(&batch).unwrap_or_default();
    CString::new(response).unwrap().into_raw()
}

/// This method gets called from the iOS NSE
///
/// # Safety
///
/// The caller must ensure that the input string was previously created by
/// `process_new_messages`.
#[no_mangle]
pub unsafe extern "C" fn free_string(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(s);
    }
}
