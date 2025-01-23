// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! User features

use anyhow::{anyhow, Context, Result};
use flutter_rust_bridge::frb;
use phnxcoreclient::{
    clients::{store::ClientRecord, CoreUser},
    Asset, UserProfile,
};
use phnxtypes::{identifiers::QualifiedUserName, messages::push_token::PushTokenOperator};
use tracing::error;

pub(crate) use phnxtypes::messages::push_token::PushToken;

/// Platform specific push token
pub enum PlatformPushToken {
    Apple(String),
    Google(String),
}

impl From<PlatformPushToken> for PushToken {
    fn from(platform_push_token: PlatformPushToken) -> Self {
        match platform_push_token {
            PlatformPushToken::Apple(token) => PushToken::new(PushTokenOperator::Apple, token),
            PlatformPushToken::Google(token) => PushToken::new(PushTokenOperator::Google, token),
        }
    }
}

/// The user of the app
///
/// Reponsible for loading or creating/registering the user.
// TODO: Most likely, it makes sense to move this to the `user_cubit` module. The loading and
// creation can be free functions there. The other functionality can be attach to the `UserCubit`.
//
// See <https://github.com/phnx-im/infra/issues/297>
pub struct User {
    pub(crate) user: CoreUser,
}

impl User {
    pub(crate) fn from_core_user(core_user: CoreUser) -> Self {
        Self { user: core_user }
    }

    /// Creates a new user with the given `user_name`.
    ///
    /// If a user with this name already exists, this will overwrite that user.
    pub async fn new(
        user_name: String,
        password: String,
        address: String,
        path: String,
        push_token: Option<PlatformPushToken>,
        display_name: Option<String>,
        profile_picture: Option<Vec<u8>>,
    ) -> Result<User> {
        let user_name: QualifiedUserName = user_name.parse()?;
        let user_profile = UserProfile::new(
            user_name.clone(),
            display_name.map(TryFrom::try_from).transpose()?,
            profile_picture.map(Asset::Value),
        );

        let user = CoreUser::new(
            user_name.clone(),
            &password,
            address,
            &path,
            push_token.map(|p| p.into()),
        )
        .await?;

        if let Err(error) = CoreUser::set_own_user_profile(&user, user_profile).await {
            error!(%error, "Could not set own user profile");
        }

        Ok(Self { user })
    }

    /// Loads the user from the given database path.
    pub async fn load_default(path: String) -> Result<User> {
        let client_record = ClientRecord::load_all_from_phnx_db(&path)?
            .pop()
            .context("No user found")?;
        let as_client_id = client_record.as_client_id;
        let user = CoreUser::load(as_client_id.clone(), &path)
            .await?
            .ok_or_else(|| {
                anyhow!(
                    "Could not load user with client_id {}",
                    as_client_id.to_string()
                )
            })?;

        Ok(Self { user: user.clone() })
    }

    /// Update the push token.
    #[frb(positional)]
    pub async fn update_push_token(&self, push_token: Option<PlatformPushToken>) -> Result<()> {
        self.user
            .update_push_token(push_token.map(|p| p.into()))
            .await?;
        Ok(())
    }

    /// Total number of unread messages across all conversations
    #[frb(getter, type_64bit_int)]
    pub async fn global_unread_messages_count(&self) -> usize {
        self.user
            .global_unread_messages_count()
            .await
            .unwrap_or_default()
    }

    /// The user name of the logged in user
    #[frb(getter)]
    pub async fn user_name(&self) -> String {
        self.user.user_name().to_string()
    }
}
