// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ffi::{CStr, CString, c_char};

use crate::background_execution::{
    IncomingNotificationContent, processing::retrieve_messages_sync,
};
use crate::logging::init_logger;

/// This method gets called from the iOS NSE
///
/// # Safety
///
/// The caller must ensure that the content is a pointer to a valid C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_new_messages(content: *const c_char) -> *mut c_char {
    assert!(!content.is_null());

    let c_str = unsafe { CStr::from_ptr(content) };

    let json_str = c_str.to_str().unwrap();
    let incoming_content: IncomingNotificationContent = serde_json::from_str(json_str).unwrap();

    init_logger(incoming_content.log_file_path);

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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_string(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(s);
    }
}
