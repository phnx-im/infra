// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use jni::{
    objects::{JClass, JString},
    sys::jstring,
    JNIEnv,
};

use crate::{
    api::mobile_logging::init_logger, background_execution::processing::retrieve_messages_sync,
};

use super::IncomingNotificationContent;

/// This methos gets called from the Android Messaging Service
#[no_mangle]
pub extern "C" fn Java_im_phnx_prototype_NativeLib_process_1new_1messages(
    mut env: JNIEnv,
    _class: JClass,
    content: JString,
) -> jstring {
    init_logger();
    // Convert Java string to Rust string
    let input: String = env
        .get_string(&content)
        .expect("Couldn't get Java string")
        .into();

    let incoming_content: IncomingNotificationContent = serde_json::from_str(&input).unwrap();

    // Retrieve messages
    let batch = retrieve_messages_sync(incoming_content.path);

    let response = serde_json::to_string(&batch).unwrap_or_default();

    // Convert Rust string back to Java string
    let output = env
        .new_string(response)
        .expect("Couldn't create Java string");
    output.into_raw()
}
