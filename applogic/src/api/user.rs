// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! User features

use std::cmp::Reverse;

use anyhow::Result;
use flutter_rust_bridge::frb;
use phnxcoreclient::{
    Asset, UserProfile,
    clients::{
        CoreUser,
        store::{ClientRecord, ClientRecordState},
    },
    open_client_db,
};
use phnxtypes::{
    identifiers::{AsClientId, QualifiedUserName},
    messages::push_token::PushTokenOperator,
};
use tracing::error;

pub(crate) use phnxtypes::messages::push_token::PushToken;
use uuid::Uuid;

use super::types::{UiClientRecord, UiUserName, UiUserProfile};

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

    /// Loads all client records from the phnx database
    ///
    /// Also tries to load user profile from the client database. In case the client database
    /// cannot be opened, the client record is skipped.
    pub async fn load_client_records(db_path: String) -> Result<Vec<UiClientRecord>> {
        let mut ui_records = Vec::new();
        for record in ClientRecord::load_all_from_phnx_db(&db_path).await? {
            match load_ui_record(&db_path, &record).await {
                Ok(record) => ui_records.push(record),
                Err(error) => {
                    error!(%error, ?record.as_client_id, "failed to load client record");
                }
            }
        }
        ui_records.reverse();
        Ok(ui_records)
    }

    pub async fn load(
        db_path: String,
        user_name: UiUserName,
        client_id: Uuid,
    ) -> anyhow::Result<Self> {
        let user_name = user_name.to_string().parse()?;
        let as_client_id = AsClientId::new(user_name, client_id);
        let user = CoreUser::load(as_client_id.clone(), &db_path).await?;
        Ok(Self { user: user.clone() })
    }

    /// Loads the default user from the given database path
    ///
    /// Returns in this order:
    /// * the default most recent user with finished registation, or if none
    /// * the most recent user with finished registration, or if none
    /// * the most recent user, if any.
    pub async fn load_default(path: String) -> Result<Option<Self>> {
        let mut records = ClientRecord::load_all_from_phnx_db(&path).await?;
        records.sort_unstable_by_key(|record| {
            let is_finished = matches!(record.client_record_state, ClientRecordState::Finished);
            Reverse((record.is_default, is_finished, record.created_at))
        });

        let mut loaded_user = None;
        for client_record in records {
            let as_client_id = client_record.as_client_id;
            match CoreUser::load(as_client_id.clone(), &path).await {
                Ok(user) => {
                    loaded_user = Some(user);
                    break;
                }
                Err(error) => error!(%as_client_id, %error, "Failed to load user"),
            };
        }

        let Some(user) = loaded_user else {
            return Ok(None);
        };

        Ok(Some(Self { user }))
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
    #[frb(getter, sync)]
    pub fn user_name(&self) -> String {
        self.user.user_name().to_string()
    }

    /// The unique identifier of the logged in user
    #[frb(getter, sync)]
    pub fn client_id(&self) -> Uuid {
        self.user.as_client_id().client_id()
    }
}

async fn load_ui_record(db_path: &str, record: &ClientRecord) -> anyhow::Result<UiClientRecord> {
    let pool = open_client_db(&record.as_client_id, db_path).await?;
    let user_name = UiUserName::from_qualified_user_name(record.as_client_id.user_name());
    let user_profile = UserProfile::load(&pool, record.as_client_id.user_name())
        .await?
        .map(|profile| UiUserProfile::from_profile(&profile));
    Ok(UiClientRecord {
        client_id: record.as_client_id.client_id(),
        created_at: record.created_at,
        user_name,
        user_profile,
        is_finished: record.client_record_state == ClientRecordState::Finished,
    })
}
