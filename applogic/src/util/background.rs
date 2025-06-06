// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    fmt,
    marker::PhantomData,
    pin::pin,
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::time;
use tokio_stream::{Stream, StreamExt};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};
use uuid::Uuid;

use super::{FibonacciBackoff, spawn_from_sync};

/// Timeout after a stream stop is not considered as error
const REGULAR_STOP_TIMEOUT: Duration = Duration::from_secs(30 * 60 * 60); // 30 minutes

/// A task that runs in the background and handles events from a stream.
///
/// The task is created from a [`BackgroundStreamContext`], which is responsible for creating the
/// stream and handling events. The task is responsible for managing the lifetime of the stream:
///
/// - stream is created by the [`BackgroundStreamContext`]
/// - stream is stopped when the app goes in the background
/// - stream is restarted when the app goes in the foreground
/// - stream is restarted on errors with a fibonacci backoff
/// - stream is restarted when it gracefully stops
pub(crate) struct BackgroundStreamTask<C, Event> {
    id: Uuid,
    name: Arc<str>,
    context: C,
    cancel: CancellationToken,
    backoff: FibonacciBackoff,
    _marker: PhantomData<Event>,
}

impl<C, Event> BackgroundStreamTask<C, Event>
where
    C: BackgroundStreamContext<Event> + Send + 'static,
    Event: fmt::Debug + Send + 'static,
{
    pub(crate) fn new(name: impl Into<String>, context: C, cancel: CancellationToken) -> Self {
        let id = Uuid::new_v4();
        let name = name.into().into();
        debug!(%name, %id, "new background stream task");
        Self {
            id,
            name,
            context,
            cancel,
            backoff: FibonacciBackoff::new(),
            _marker: PhantomData,
        }
    }

    pub(crate) fn spawn(mut self) {
        spawn_from_sync(async move {
            loop {
                let started_at = Instant::now();
                let res = self.run().await;

                // stop handler on cancellation
                if self.cancel.is_cancelled() {
                    info!(name = %self.name, id = %self.id, "background stream stopped");
                    return;
                }

                // wait until the app is in foreground
                self.context.in_foreground().await;

                if let Err(error) = res {
                    // if failed faster than regular stop, retry with backoff
                    let timeout = if started_at.elapsed() < REGULAR_STOP_TIMEOUT {
                        self.backoff.next_backoff()
                    } else {
                        self.backoff.reset();
                        Duration::default()
                    };

                    error!(
                        name = %self.name,
                        id = %self.id,
                        %error,
                        retry_in =? timeout,
                        "background stream failed"
                    );
                    time::sleep(timeout).await;
                } else {
                    // otherwise, reset backoff and reconnect
                    self.backoff.reset();
                }
            }
        });
    }

    async fn run(&mut self) -> anyhow::Result<()> {
        let mut stream = pin!(self.context.create_stream().await?);
        info!(name = %self.name, id = %self.id, "background stream started");
        loop {
            let event = tokio::select! {
                event = stream.next() => event,
                _ = self.cancel.cancelled() => return Ok(()),
                _ = self.context.in_background() => return Ok(()),
            };

            match event {
                Some(event) => {
                    debug!(name = %self.name, id = %self.id, ?event, "received event");
                    self.context.handle_event(event).await
                }
                None => return Ok(()), // regular stop
            }

            // reset backoff after event handled successfully
            self.backoff.reset();
        }
    }
}

/// Context for a background stream task.
///
/// Responsible for creating the stream and handling events.
pub(crate) trait BackgroundStreamContext<Event>: Send + Clone {
    /// Create the backtrack stream
    fn create_stream(
        &self,
    ) -> impl Future<Output = anyhow::Result<impl Stream<Item = Event> + Send>> + Send;

    /// Handle a stream event
    fn handle_event(&self, event: Event) -> impl Future<Output = ()> + Send;

    /// Resolves when the app is in the foreground
    fn in_foreground(&self) -> impl Future<Output = ()> + Send;

    /// Resolves when the app is in the background
    fn in_background(&self) -> impl Future<Output = ()> + Send;
}
