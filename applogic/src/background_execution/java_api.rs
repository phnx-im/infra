// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use jni::{
    JNIEnv,
    objects::{JClass, JObject, JString},
    sys::jstring,
};

use crate::{background_execution::processing::retrieve_messages_sync, logging::init_logger};

use super::IncomingNotificationContent;

#[unsafe(export_name = "Java_im_phnx_prototype_NativeLib_registerJavaVm")]
pub extern "C" fn register_java_vm(env: JNIEnv, _class: JClass, jni_notifications_class: JObject) {
    notifications::register_java_vm(env, jni_notifications_class);
}

/// This methos gets called from the Android Messaging Service
#[unsafe(export_name = "Java_im_phnx_prototype_NativeLib_process_1new_1messages")]
pub extern "C" fn process_new_messages(
    mut env: JNIEnv,
    _class: JClass,
    content: JString,
) -> jstring {
    // Convert Java string to Rust string
    let input: String = env
        .get_string(&content)
        .expect("Couldn't get Java string")
        .into();

    let incoming_content: IncomingNotificationContent = serde_json::from_str(&input).unwrap();

    init_logger(incoming_content.log_file_path.clone());
    tracing::warn!(incoming_content.log_file_path, "init_logger");

    // Retrieve messages
    let batch = retrieve_messages_sync(incoming_content.path);

    let response = serde_json::to_string(&batch).unwrap_or_default();

    // Convert Rust string back to Java string
    let output = env
        .new_string(response)
        .expect("Couldn't create Java string");
    output.into_raw()
}
