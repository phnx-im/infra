// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later
//
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{SseEncode, StreamSink};

/// A shareable collection of [`StreamSink`]
///
/// Allows to lock the shared state for only a short period of time.
#[derive(Default, Clone)]
pub(crate) struct SharedCubitSinks<T> {
    inner: Arc<SinksInner<T>>,
}

#[derive(Default)]
struct SinksInner<T> {
    sinks: Mutex<Vec<StreamSink<T>>>,
    is_closed: AtomicBool,
}

impl<T> SharedCubitSinks<T>
where
    T: SseEncode + Clone,
{
    pub(crate) fn is_closed(&self) -> bool {
        self.inner.is_closed.load(Ordering::SeqCst)
    }

    pub(crate) fn close(&self) {
        self.inner.is_closed.store(true, Ordering::SeqCst);
    }

    pub(crate) async fn push(&self, sink: StreamSink<T>) {
        if self.is_closed() {
            return;
        }
        log::info!("Adding sink");
        self.inner.sinks.lock().await.push(sink);
    }

    pub(crate) async fn emit(&self, state: T) {
        if self.is_closed() {
            return;
        }
        self.inner.sinks.lock().await.retain(|sink| {
            log::info!("Emitting state to sink");
            sink.add(state.clone()).is_ok()
        });
    }
}
