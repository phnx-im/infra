// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    collections::{HashMap, hash_map},
    pin::pin,
    sync::Arc,
};

use flutter_rust_bridge::frb;
use phnxcommon::identifiers::UserId;
use phnxcoreclient::{
    DisplayName, UserProfile,
    clients::CoreUser,
    store::{Store, StoreEntityId, StoreOperation},
};
use tokio::sync::watch;
use tokio_stream::StreamExt;
use tokio_util::sync::{CancellationToken, DropGuard};

use crate::{
    StreamSink,
    util::{Cubit, CubitCore, spawn_from_sync},
};

use super::{
    types::{ImageData, UiUserId, UiUserProfile},
    user_cubit::UserCubitBase,
};

#[derive(Debug, Clone, Default)]
#[frb(opaque)]
pub struct ContactsState {
    inner: Arc<ContactsStateInner>,
}

#[derive(Debug, Clone, Default)]
#[frb(ignore)]
pub(crate) struct ContactsStateInner {
    profiles: HashMap<UserId, UiUserProfile>,
}

impl ContactsStateInner {
    /// Returns `true` if the profile was changed, otherwise `false`.
    ///
    /// A profile is not changed if the profile is the same as the current one.
    fn set_profile(&mut self, user_id: UserId, profile: UserProfile) -> bool {
        let profile = UiUserProfile::from_profile(profile);
        match self.profiles.entry(user_id) {
            hash_map::Entry::Occupied(mut entry) => {
                if entry.get() != &profile {
                    entry.insert(profile);
                    true
                } else {
                    false
                }
            }
            hash_map::Entry::Vacant(entry) => {
                entry.insert(profile);
                true
            }
        }
    }
}

pub struct ContactsCubitBase {
    core: CubitCore<ContactsState>,
    store: CoreUser,
    _cancel: DropGuard,
}

impl ContactsCubitBase {
    #[frb(sync)]
    pub fn new(user_cubit: &UserCubitBase) -> Self {
        let store = user_cubit.core_user.clone();

        let core = CubitCore::new();

        let cancel = CancellationToken::new();
        spawn_process_store_notifications(store.clone(), core.state_tx().clone(), cancel.clone());

        Self {
            core,
            store,
            _cancel: cancel.drop_guard(),
        }
    }

    // Cubit interface

    pub fn close(&mut self) {
        self.core.close();
    }

    #[frb(getter, sync)]
    pub fn is_closed(&self) -> bool {
        self.core.is_closed()
    }

    #[frb(getter, sync)]
    pub fn state(&self) -> ContactsState {
        self.core.state()
    }

    pub async fn stream(&mut self, sink: StreamSink<ContactsState>) {
        self.core.stream(sink).await;
    }

    // Cubit methods

    #[frb(sync)]
    pub fn profile(&self, user_id: Option<UiUserId>) -> UiUserProfile {
        let user_id: UserId = user_id
            .map(From::from)
            .unwrap_or_else(|| self.store.user_id().clone());
        let profile = self
            .core
            .state_tx()
            .borrow()
            .inner
            .profiles
            .get(&user_id)
            .cloned();
        match profile {
            Some(profile) => profile,
            None => {
                // spawn loading profile and return default profile
                self.spawn_load_profile(user_id.clone());
                let default_profile = UserProfile::from_user_id(&user_id);
                UiUserProfile::from_profile(default_profile)
            }
        }
    }

    #[frb(sync)]
    pub fn display_name(&self, user_id: Option<UiUserId>) -> String {
        let user_id: UserId = user_id
            .map(From::from)
            .unwrap_or_else(|| self.store.user_id().clone());
        let display_name = self
            .core
            .state_tx()
            .borrow()
            .inner
            .profiles
            .get(&user_id)
            .map(|profile| profile.display_name.clone());
        match display_name {
            Some(display_name) => display_name,
            None => {
                // spawn loading profile and return default display name
                self.spawn_load_profile(user_id.clone());
                DisplayName::from_user_id(&user_id).into_string()
            }
        }
    }

    #[frb(sync)]
    pub fn profile_picture(&self, user_id: Option<UiUserId>) -> Option<ImageData> {
        let user_id: UserId = user_id
            .map(From::from)
            .unwrap_or_else(|| self.store.user_id().clone());
        let profile_picture = self
            .core
            .state_tx()
            .borrow()
            .inner
            .profiles
            .get(&user_id)
            .map(|profile| profile.profile_picture.clone());
        match profile_picture {
            Some(profile_picture) => profile_picture,
            None => {
                // spawn loading profile
                self.spawn_load_profile(user_id.clone());
                None
            }
        }
    }

    #[frb(ignore)]
    fn spawn_load_profile(&self, user_id: UserId) {
        let store = self.store.clone();
        let state_tx = self.core.state_tx().clone();
        spawn_from_sync(async move {
            let profile = store.user_profile(&user_id).await;
            state_tx.send_if_modified(|state| {
                let inner = Arc::make_mut(&mut state.inner);
                inner.set_profile(user_id, profile)
            });
        });
    }
}

fn spawn_process_store_notifications(
    store: impl Store + 'static,
    state_tx: watch::Sender<ContactsState>,
    cancel: CancellationToken,
) {
    spawn_from_sync(process_store_notifications(store, state_tx, cancel));
}

async fn process_store_notifications(
    store: impl Store,
    state_tx: watch::Sender<ContactsState>,
    cancel: CancellationToken,
) {
    let mut store_notifications = pin!(store.subscribe());
    loop {
        // wait for the next notification or cancellation
        let notifications = tokio::select! {
           notifications = store_notifications.next() => notifications,
            _ = cancel.cancelled() => return,
        };
        let Some(notifications) = notifications else {
            return;
        };

        // collect changed or removed profiles
        let mut changed_profiles = Vec::new();
        for (entity_id, op) in notifications.ops.iter() {
            let StoreEntityId::User(user_id) = entity_id else {
                continue;
            };

            let is_loaded = state_tx.borrow().inner.profiles.contains_key(user_id);
            if !is_loaded {
                continue;
            }

            // We consider Add/Update to be of higher precedence than Remove
            if op.contains(StoreOperation::Add) || op.contains(StoreOperation::Update) {
                changed_profiles.push((user_id, Some(store.user_profile(user_id).await)));
            } else if op.contains(StoreOperation::Remove) {
                changed_profiles.push((user_id, None));
            }
        }

        // update the state
        state_tx.send_if_modified(|state| {
            let inner = Arc::make_mut(&mut state.inner);
            let mut modified = false;
            for (user_id, profile) in changed_profiles {
                if let Some(profile) = profile {
                    if inner.set_profile(user_id.clone(), profile) {
                        modified = true;
                    }
                } else {
                    inner.profiles.remove(user_id);
                    modified = true;
                }
            }
            modified
        });
    }
}
