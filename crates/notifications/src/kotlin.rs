use std::sync::OnceLock;

use anyhow::{Context, bail};
use jni::{
    JNIEnv, JavaVM,
    errors::Result as JNIResult,
    objects::{GlobalRef, JObject, JString, JValue},
};
use tracing::error;

static JAVA_VM: OnceLock<JavaVM> = OnceLock::new();
static JNI_NOTIFICATIONS_CLASS: OnceLock<GlobalRef> = OnceLock::new();

pub fn register_java_vm(env: JNIEnv, jni_notifications_class: JObject) {
    let vm = env.get_java_vm().expect("failed to get JavaVM");
    let _ = JAVA_VM.set(vm);

    let global_ref = env
        .new_global_ref(jni_notifications_class)
        .expect("failed to create global ref");

    let _ = JNI_NOTIFICATIONS_CLASS.set(global_ref);
}

pub(crate) fn send_notification(identifier: &str, title: &str, message: &str) {
    if let Err(error) = try_send_notification(identifier, title, message) {
        error!(%error, "failed to show notification");
    }
}

/// Shows a notification on Android via JNI.
pub(crate) fn try_send_notification(
    identifier: &str,
    title: &str,
    message: &str,
) -> anyhow::Result<()> {
    let vm = JAVA_VM
        .get()
        .context("JavaVM not initialized; did you call register_java_vm?")?;
    let jni_notifications_class = JNI_NOTIFICATIONS_CLASS
        .get()
        .context("JniNotifcations class not initialized; did you call register_java_vm?")?;

    let mut env = vm
        .attach_current_thread()
        .context("failed to attach current thread")?;

    let jidentifier = env
        .new_string(identifier)
        .context("failed to create identifier string")?;
    let jtitle = env
        .new_string(title)
        .context("failed to create title string")?;
    let jmessage = env
        .new_string(message)
        .context("failed to create message string")?;

    env.call_method(
        jni_notifications_class.as_obj(),
        "showNotification",
        "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)V",
        &[(&jidentifier).into(), (&jtitle).into(), (&jmessage).into()],
    )
    .context("failed to call showNotification")?;

    Ok(())
}

pub(crate) fn remove_notifications(identifiers: &[impl AsRef<str>]) {
    if let Err(error) = try_remove_notifications(identifiers) {
        error!(%error, "failed to remove notifications");
    }
}

fn try_remove_notifications(identifiers: &[impl AsRef<str>]) -> anyhow::Result<()> {
    let vm = JAVA_VM
        .get()
        .context("JavaVM not initialized; did you call register_java_vm?")?;
    let jni_notifications_class = JNI_NOTIFICATIONS_CLASS
        .get()
        .context("JniNotifcations class not initialized; did you call register_java_vm?")?;

    let mut env = vm
        .attach_current_thread()
        .context("failed to attach current thread")?;

    let identifiers = java_string_array_from_slice(&mut env, identifiers)?;

    env.call_method(
        jni_notifications_class.as_obj(),
        "cancelNotifications",
        "(Ljava/util/ArrayList;)V",
        &[(&identifiers).into()],
    )
    .context("failed to call showNotification")?;

    Ok(())
}

pub(crate) fn active_notifications() -> Vec<String> {
    try_active_notifications()
        .inspect_err(|error| error!(%error, "failed to get active notifications"))
        .unwrap_or_default()
}

fn try_active_notifications() -> anyhow::Result<Vec<String>> {
    let vm = JAVA_VM
        .get()
        .context("JavaVM not initialized; did you call register_java_vm?")?;
    let jni_notifications_class = JNI_NOTIFICATIONS_CLASS
        .get()
        .context("JniNotifcations class not initialized; did you call register_java_vm?")?;

    let mut env = vm
        .attach_current_thread()
        .context("failed to attach current thread")?;

    let result = env
        .call_method(
            jni_notifications_class.as_obj(),
            "getActiveNotifications",
            "()Ljava/util/ArrayList;",
            &[],
        )
        .context("failed to call activeNotifications")?
        .l()?;

    Ok(java_string_array_to_vec(&mut env, &result)?)
}

/// Create a `java.util.ArrayList<String>` from a slice of Rust strings.
fn java_string_array_from_slice<'local>(
    env: &mut JNIEnv<'local>,
    items: &[impl AsRef<str>],
) -> JNIResult<JObject<'local>> {
    // Create an empty `ArrayList`: `new ArrayList<>()`
    let array_list_class = env.find_class("java/util/ArrayList")?;
    let array_list_obj = env.new_object(array_list_class, "()V", &[])?;

    // For each Rust string, convert to a Java string and call ArrayList.add(...)
    for s in items {
        let jstr = env.new_string(s.as_ref())?;
        env.call_method(
            &array_list_obj,
            "add",
            "(Ljava/lang/Object;)Z",
            &[JValue::Object(&jstr)],
        )?;
    }

    Ok(array_list_obj)
}

fn java_string_array_to_vec(env: &mut JNIEnv, jarraylist: &JObject) -> JNIResult<Vec<String>> {
    let len = env.call_method(jarraylist, "size", "()I", &[])?.i()?;
    let mut res = Vec::with_capacity(len.try_into().unwrap_or(0));
    for idx in 0..len {
        let jstring = env
            .call_method(jarraylist, "get", "(I)Ljava/lang/Object;", &[idx.into()])?
            .l()?;
        let jstring = JString::from(jstring);
        let value: String = env.get_string(&jstring)?.into();
        res.push(value);
    }
    Ok(res)
}
