// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use flutter_rust_bridge::{DartFnFuture, DartOpaque, frb};
use phnxcoreclient::ConversationId;

/// Encapsulates the navigation cubit from the Dart side.
///
/// The App navigation is implemented on the Dart side. This wrapper allows to access specific
/// state properties from Rust.
#[derive(Clone)]
pub struct DartNavigation {
    navigation_cubit: DartOpaque,
    callbacks: Arc<Callbacks>,
}

struct Callbacks {
    current_conversation:
        Box<dyn Fn(DartOpaque) -> DartFnFuture<Option<ConversationId>> + Send + Sync>,
}

static_assertions::assert_impl_all!(DartNavigation: Send, Sync);

impl DartNavigation {
    /// Wraps the navigation cubit from the Dart side.
    ///
    /// The provided callbacks are called on the `navigation_cubit` parameter.
    #[frb(sync)]
    pub fn new(
        navigation_cubit: DartOpaque,
        current_conversation_callback: impl Fn(DartOpaque) -> DartFnFuture<Option<ConversationId>>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        Self {
            navigation_cubit,
            callbacks: Callbacks {
                current_conversation: Box::new(current_conversation_callback),
            }
            .into(),
        }
    }

    /// Returns the conversation ID of the currently open conversation, if any.
    pub async fn current_conversation_id(&self) -> Option<ConversationId> {
        (self.callbacks.current_conversation)(self.navigation_cubit.clone()).await
    }
}
