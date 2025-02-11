// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::identifiers::QualifiedUserName;
use rusqlite::{params, Connection, OptionalExtension};
use sqlx::{query, query_as, SqlitePool};
use tracing::error;

use crate::{store::StoreNotifier, utils::persistence::Storable, Asset, DisplayName, UserProfile};

impl Storable for UserProfile {
    const CREATE_TABLE_STATEMENT: &'static str = "CREATE TABLE IF NOT EXISTS users (
                user_name TEXT PRIMARY KEY,
                display_name TEXT,
                profile_picture BLOB
            );";

    fn from_row(row: &rusqlite::Row) -> anyhow::Result<Self, rusqlite::Error> {
        let user_name = row.get(0)?;
        let display_name_option = row.get(1)?;
        let profile_picture_option = row.get(2)?;
        Ok(UserProfile {
            user_name,
            display_name_option,
            profile_picture_option,
        })
    }
}

struct SqlUserProfile {
    user_name: QualifiedUserName,
    display_name: Option<DisplayName>,
    profile_picture: Option<Asset>,
}

impl From<SqlUserProfile> for UserProfile {
    fn from(
        SqlUserProfile {
            user_name,
            display_name,
            profile_picture,
        }: SqlUserProfile,
    ) -> Self {
        Self {
            user_name,
            display_name_option: display_name,
            profile_picture_option: profile_picture,
        }
    }
}

impl UserProfile {
    pub fn load(
        connection: &Connection,
        user_name: &QualifiedUserName,
    ) -> Result<Option<Self>, rusqlite::Error> {
        let mut statement = connection.prepare(
            "SELECT user_name, display_name, profile_picture FROM users WHERE user_name = ?",
        )?;
        let user = statement
            .query_row(params![user_name.to_string()], Self::from_row)
            .optional()?;

        if let Some(user_profile) = &user {
            if user_name != user_profile.user_name() {
                // This should never happen, but if it does, we want to know about it.
                error!(
                    expected =% user_name,
                    actual =% user_profile.user_name(),
                    "User name mismatch",
                );
            }
        }
        Ok(user)
    }

    pub(crate) async fn load_2(
        db: &SqlitePool,
        user_name: &QualifiedUserName,
    ) -> sqlx::Result<Option<Self>> {
        query_as!(
            SqlUserProfile,
            r#"SELECT
                user_name AS "user_name: _",
                display_name AS "display_name: _",
                profile_picture AS "profile_picture: _"
            FROM users WHERE user_name = ?"#,
            user_name,
        )
        .fetch_optional(db)
        .await
        .map(|record| record.map(From::from))
    }

    /// Store the user's profile in the database. This will overwrite any existing profile.
    pub(crate) fn store_or_replace(
        connection: &Connection,
        notifier: &mut StoreNotifier,
        user_name: QualifiedUserName,
        display_name_option: Option<DisplayName>,
        profile_picture_option: Option<Asset>,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "INSERT OR REPLACE INTO users (user_name, display_name, profile_picture) VALUES (?, ?, ?)",
            params![
                user_name.to_string(),
                display_name_option,
                profile_picture_option
            ],
        )?;
        notifier.update(user_name);
        Ok(())
    }

    pub(crate) async fn store_or_replace_2(
        db: &SqlitePool,
        notifier: &mut StoreNotifier,
        user_name: QualifiedUserName,
        display_name: Option<DisplayName>,
        profile_picture: Option<Asset>,
    ) -> sqlx::Result<()> {
        query!(
            "INSERT OR REPLACE INTO users (user_name, display_name, profile_picture)
            VALUES (?, ?, ?)",
            user_name,
            display_name,
            profile_picture,
        )
        .execute(db)
        .await?;
        notifier.update(user_name);
        Ok(())
    }

    /// Update the user's display name and profile picture in the database. To store a new profile,
    /// use [`register_as_conversation_participant`] instead.
    pub(crate) fn update(
        &self,
        connection: &Connection,
        notifier: &mut StoreNotifier,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "UPDATE users SET display_name = ?2, profile_picture = ?3 WHERE user_name = ?1",
            params![
                self.user_name.to_string(),
                self.display_name_option,
                self.profile_picture_option
            ],
        )?;
        notifier.update(self.user_name.clone());
        Ok(())
    }

    pub(crate) async fn update_2(
        &self,
        db: &SqlitePool,
        notifier: &mut StoreNotifier,
    ) -> sqlx::Result<()> {
        query!(
            "UPDATE users SET display_name = ?2, profile_picture = ?3 WHERE user_name = ?1",
            self.user_name,
            self.display_name_option,
            self.profile_picture_option
        )
        .execute(db)
        .await?;
        notifier.update(self.user_name.clone());
        Ok(())
    }

    /// Stores this new [`UserProfile`] if one doesn't already exist.
    pub(crate) fn store_or_ignore(
        &self,
        connection: &Connection,
        notifier: &mut StoreNotifier,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "INSERT OR IGNORE INTO users (user_name, display_name, profile_picture) VALUES (?, ?, ?)",
            params![
                self.user_name.to_string(),
                self.display_name_option,
                self.profile_picture_option
            ],
        )?;
        // TODO: We can skip this notification if the user profile was already stored.
        notifier.add(self.user_name.clone());
        Ok(())
    }

    pub(crate) async fn store_or_ignore_2(
        db: &SqlitePool,
        notifier: &mut StoreNotifier,
        user_name: QualifiedUserName,
        display_name: Option<DisplayName>,
        profile_picture: Option<Asset>,
    ) -> sqlx::Result<()> {
        query!(
            "INSERT OR IGNORE INTO users (user_name, display_name, profile_picture)
            VALUES (?, ?, ?)",
            user_name,
            display_name,
            profile_picture,
        )
        .execute(db)
        .await?;
        // TODO: We can skip this notification if the user profile was already stored.
        notifier.add(user_name);
        Ok(())
    }
}
