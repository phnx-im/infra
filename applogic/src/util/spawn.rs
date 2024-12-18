// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use flutter_rust_bridge::{BaseAsyncRuntime, JoinHandle};
use std::future::Future;

use crate::FLUTTER_RUST_BRIDGE_HANDLER;

/// Spawn a future from a synchronous function.
///
/// Note: Spawning a task via [`flutter_rust_bridge::spawn`] is only possible from async functions.
#[track_caller]
pub(crate) fn spawn_from_sync<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    FLUTTER_RUST_BRIDGE_HANDLER.async_runtime().spawn(future)
}
