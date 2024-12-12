// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use anyhow::bail;
use flutter_rust_bridge::frb;
use log::info;
use phnxcoreclient::clients::CoreUser;
use phnxcoreclient::{Asset, UserProfile};
use phnxtypes::identifiers::QualifiedUserName;
use tokio::sync::RwLock;

use crate::util::spawn_from_sync;

use super::{StreamSink, User};

/// Logged in user
///
/// Opaque, cheaply clonable, copy-on-write type
///
/// Note: This has a prefix `Ui` to avoid conflicts with the `User`.
//
// TODO: Currently, frb does not support exposing eq and hash to Dart. When it is possible, we
// should do it, to minimize the amount of UI rebuilds in Flutter.
//
// See:
// * <https://github.com/phnx-im/infra/issues/247>
// * <https://github.com/fzyzcjy/flutter_rust_bridge/issues/2238>
#[frb(opaque)]
#[derive(Debug, Clone)]
pub struct UiUser {
    inner: Arc<UiUserInner>,
}

#[derive(Debug)]
struct UiUserInner {
    user_name: QualifiedUserName,
    profile: Option<UserProfile>,
}

impl UiUser {
    fn new(user_name: QualifiedUserName, profile: Option<UserProfile>) -> Self {
        let inner = Arc::new(UiUserInner { user_name, profile });
        Self { inner }
    }

    /// Loads the user profile in the background
    fn spawn_load(this: Arc<RwLock<Self>>, core_user: CoreUser) {
        spawn_from_sync(async move {
            match core_user.own_user_profile().await {
                Ok(profile) => {
                    let mut state = this.write().await;
                    *state = UiUser::new(state.inner.user_name.clone(), Some(profile));
                }
                Err(error) => {
                    log::error!("Could not load own user profile: {:?}", error);
                }
            }
        });
    }

    #[frb(getter, sync)]
    pub fn user_name(&self) -> String {
        self.inner.user_name.to_string()
    }

    #[frb(getter, sync)]
    pub fn display_name(&self) -> Option<String> {
        let profile = self.inner.profile.as_ref()?;
        Some(profile.display_name()?.to_string())
    }

    #[frb(getter, sync)]
    pub fn profile_picture(&self) -> Option<Vec<u8>> {
        let profile = self.inner.profile.as_ref()?;
        Some(profile.profile_picture()?.value()?.to_vec())
    }
}

/// Provides access to the logged in user and their profile.
///
/// Note: this has a suffix `Base` because the corresponding Dart class does not implement
/// `StateStreamableSource`, and therefore to impemlement it we need to wrap it Dart.
#[frb(opaque)]
pub struct UserCubitBase {
    state: Arc<RwLock<UiUser>>,
    sinks: Option<Vec<StreamSink<UiUser>>>,
    core_user: CoreUser,
}

impl UserCubitBase {
    #[frb(sync)]
    pub fn new(user: &User) -> Self {
        info!("UserCubitBase::new");

        let core_user = user.user.clone();
        let state = Arc::new(RwLock::new(UiUser::new(core_user.user_name(), None)));

        UiUser::spawn_load(state.clone(), core_user.clone());

        // TODO: Subscribe to the change notifications from the core user.
        // See <https://github.com/phnx-im/infra/issues/254>

        Self {
            state,
            sinks: Some(Default::default()),
            core_user,
        }
    }

    fn emit(&mut self, state: UiUser) {
        if let Some(sinks) = &mut self.sinks {
            sinks.retain(|sink| sink.add(state.clone()).is_ok());
        }
    }

    // Cubit inteface

    pub fn close(&mut self) {
        self.sinks = None;
    }

    #[frb(getter, sync)]
    pub fn is_closed(&self) -> bool {
        self.sinks.is_none()
    }

    #[frb(getter, sync)]
    pub fn state(&self) -> UiUser {
        self.state.blocking_read().clone()
    }

    pub fn stream(&mut self, sink: StreamSink<UiUser>) {
        if let Some(sinks) = &mut self.sinks {
            sinks.push(sink);
        }
    }

    // Cubit methods

    /// Set the display name and/or profile picture of the user.
    pub async fn set_profile(
        &mut self,
        display_name: Option<String>,
        profile_picture: Option<Vec<u8>>,
    ) -> anyhow::Result<()> {
        let display_name = display_name.map(TryFrom::try_from).transpose()?;
        let profile_picture = profile_picture.map(Asset::Value);
        let user = {
            let mut state = self.state.write().await;
            let Some(user_profile) = &state.inner.profile else {
                bail!("Can't set display name for user without a profile");
            };
            let mut user_profile = user_profile.clone();
            if let Some(value) = display_name {
                user_profile.set_display_name(Some(value));
            }
            if let Some(value) = profile_picture {
                user_profile.set_profile_picture(Some(value));
            }
            self.core_user
                .set_own_user_profile(user_profile.clone())
                .await?;
            let user = UiUser::new(state.inner.user_name.clone(), Some(user_profile.clone()));
            *state = user.clone();
            user
        };
        self.emit(user);
        Ok(())
    }
}
