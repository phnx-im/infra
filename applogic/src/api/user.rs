// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::{anyhow, Result};
use flutter_rust_bridge::frb;
use phnxcoreclient::{
    clients::{store::ClientRecord, CoreUser},
    Asset, UserProfile,
};
use phnxtypes::{
    identifiers::{QualifiedUserName, SafeTryInto},
    messages::push_token::PushTokenOperator,
};
use tracing::error;

pub(crate) use phnxtypes::messages::push_token::PushToken;

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

pub struct User {
    pub(crate) user: CoreUser,
}

impl User {
    pub(crate) fn from_core_user(core_user: CoreUser) -> Self {
        Self { user: core_user }
    }

    pub async fn new(
        user_name: String,
        password: String,
        address: String,
        path: String,
        push_token: Option<PlatformPushToken>,
        display_name: Option<String>,
        profile_picture: Option<Vec<u8>>,
    ) -> Result<User> {
        let user_name: QualifiedUserName = SafeTryInto::try_into(user_name)?;
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

        Ok(Self {
            user: user.clone(),
            // app_state: AppState::new(user),
            // notification_hub: NotificationHub::<DartNotifier>::default(),
        })
    }

    pub async fn load_default(path: String) -> Result<User> {
        let client_record = ClientRecord::load_all_from_phnx_db(&path)?
            .pop()
            .ok_or_else(|| anyhow!("No user found."))?;
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

    #[frb(getter)]
    pub async fn global_unread_messages_count(&self) -> u32 {
        self.user
            .global_unread_messages_count()
            .await
            .unwrap_or_default()
    }

    #[frb(getter)]
    pub async fn user_name(&self) -> String {
        self.user.user_name().to_string()
    }
}
