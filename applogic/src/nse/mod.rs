use serde::{Deserialize, Serialize};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

#[derive(Serialize, Deserialize)]
struct IncomingNotificationContent {
    title: String,
    body: String,
    data: String,
}

#[derive(Serialize, Deserialize)]
struct NotificationBatch {
    removals: Vec<String>,
    additions: Vec<NotificationContent>,
}

#[derive(Serialize, Deserialize)]
struct NotificationContent {
    identifier: String,
    title: String,
    body: String,
}

#[no_mangle]
pub extern "C" fn process_new_messages(content: *const c_char) -> *mut c_char {
    let c_str = unsafe {
        assert!(!content.is_null());
        CStr::from_ptr(content)
    };

    let json_str = c_str.to_str().unwrap();
    let incoming_content: IncomingNotificationContent = serde_json::from_str(json_str).unwrap();

    // Test notifictaions only for now
    let (removals, additions) = match &incoming_content.data[..] {
        "add" => (
            vec!["documentation".to_string()],
            vec![
                NotificationContent {
                    identifier: "documentation".to_string(),
                    title: "Added the placeholder notification".to_string(),
                    body: "Add operation".to_string(),
                },
                NotificationContent {
                    identifier: "placeholder".to_string(),
                    title: "Placeholder notification".to_string(),
                    body: "This is a placeholder notification".to_string(),
                },
            ],
        ),
        "remove" => (
            vec!["documentation".to_string(), "placeholder".to_string()],
            vec![NotificationContent {
                identifier: "documentation".to_string(),
                title: "Removed the placeholder notification".to_string(),
                body: "Remove operation".to_string(),
            }],
        ),
        _ => (
            vec!["documentation".to_string()],
            vec![NotificationContent {
                identifier: "documentation".to_string(),
                title: "Could not process command".to_string(),
                body: format!("Unknown command: {}", incoming_content.data),
            }],
        ),
    };

    let batch = NotificationBatch {
        removals,
        additions,
    };

    let response = serde_json::to_string(&batch).unwrap();
    CString::new(response).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn free_string(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(s);
    }
}
