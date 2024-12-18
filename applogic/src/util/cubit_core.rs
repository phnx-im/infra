// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;

use log::trace;
use tokio::sync::{mpsc, watch};
use tokio_util::sync::CancellationToken;

use crate::{SseEncode, StreamSink};

use super::spawn_from_sync;

pub(crate) trait Cubit {
    type State;

    fn close(&mut self);

    fn is_closed(&self) -> bool;

    fn state(&self) -> Self::State;

    async fn stream(&mut self, sink: StreamSink<Self::State>);
}

pub(crate) struct CubitCore<S> {
    state_tx: watch::Sender<S>,
    sinks_tx: mpsc::Sender<StreamSink<S>>,
    cancel: CancellationToken,
}

impl<S> Drop for CubitCore<S> {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

impl<S: Clone> Cubit for CubitCore<S> {
    type State = S;

    fn is_closed(&self) -> bool {
        self.cancel.is_cancelled()
    }

    fn close(&mut self) {
        self.cancel.cancel();
    }

    fn state(&self) -> S {
        self.state_tx.borrow().clone()
    }

    async fn stream(&mut self, sink: StreamSink<S>) {
        if self.sinks_tx.send(sink).await.is_err() {
            self.close();
        }
    }
}

impl<S> CubitCore<S>
where
    S: SseEncode + Default + Clone + Send + Sync + fmt::Debug + 'static,
{
    pub(crate) fn new() -> Self {
        let (state_tx, state_rx) = watch::channel(S::default());
        let (sinks_tx, sinks_rx) = mpsc::channel(16);
        let cancel = CancellationToken::new();

        spawn_from_sync(Self::emitter_loop(state_rx, sinks_rx, cancel.clone()));

        Self {
            state_tx,
            sinks_tx,
            cancel,
        }
    }

    pub(crate) fn state_tx(&self) -> &watch::Sender<S> {
        &self.state_tx
    }

    pub(crate) fn borrow_state(&self) -> watch::Ref<'_, S> {
        self.state_tx.borrow()
    }

    pub(crate) fn cancellation_token(&self) -> &CancellationToken {
        &self.cancel
    }

    async fn emitter_loop(
        mut state_rx: watch::Receiver<S>,
        mut sinks_rx: mpsc::Receiver<StreamSink<S>>,
        stop: CancellationToken,
    ) {
        let mut sinks = Vec::new();
        loop {
            tokio::select! {
                sink = sinks_rx.recv() => {
                    let Some(sink) = sink else { return };
                    sinks.push(sink);
                },
                changed = state_rx.changed() => {
                    if changed.is_err() {
                        return;
                    };
                    let state = state_rx.borrow().clone();
                    trace!("emitting new state, sinks = {}, state = {:?}", sinks.len(), state);
                    sinks.retain(|sink| sink.add(state.clone()).is_ok());
                },
                _ = stop.cancelled() => {
                    return;
                }
            }
        }
    }
}
