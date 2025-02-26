// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::identifiers::QualifiedUserName;
use rusqlite::{Connection, OptionalExtension, params};
use tracing::error;

use crate::{UserProfile, store::StoreNotifier, utils::persistence::Storable};

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

    /// Stores this new [`UserProfile`] if one doesn't already exist.
    pub(crate) fn store(
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

    /// Stores this new [`UserProfile`].
    ///
    /// Replaces the existing user profile if one exists.
    pub(crate) fn upsert(
        &self,
        connection: &Connection,
        notifier: &mut StoreNotifier,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "INSERT OR REPLACE INTO users (user_name, display_name, profile_picture) VALUES (?, ?, ?)",
            params![
                self.user_name.to_string(),
                self.display_name_option,
                self.profile_picture_option
            ],
        )?;
        notifier.update(self.user_name.clone());
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
}

#[cfg(test)]
mod tests {
    use crate::Asset;

    use super::*;

    fn test_connection() -> rusqlite::Connection {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        connection
            .execute_batch(UserProfile::CREATE_TABLE_STATEMENT)
            .unwrap();
        connection
    }

    fn test_profile() -> UserProfile {
        UserProfile::new(
            "alice@localhost".parse().unwrap(),
            Some("Alice".to_string().try_into().unwrap()),
            Some(Asset::Value(vec![1, 2, 3])),
        )
    }

    #[test]
    fn store_load() -> anyhow::Result<()> {
        let connection = test_connection();
        let mut notifier = StoreNotifier::noop();

        let profile = test_profile();

        profile.store(&connection, &mut notifier)?;
        let loaded = UserProfile::load(&connection, &profile.user_name)?.expect("profile exists");
        assert_eq!(loaded, profile);

        let mut new_profile = profile.clone();
        new_profile.set_display_name(Some("Alice In Wonderland".to_string().try_into()?));
        new_profile.set_profile_picture(None);

        // store ignores the new profile if the user already exists
        new_profile.store(&connection, &mut notifier)?;
        let loaded = UserProfile::load(&connection, &profile.user_name)?.expect("profile exists");
        assert_eq!(loaded, profile);
        assert_ne!(loaded, new_profile);

        // upsert/load works
        new_profile.upsert(&connection, &mut notifier)?;
        let loaded = UserProfile::load(&connection, &profile.user_name)?.expect("profile exists");
        assert_ne!(loaded, profile);
        assert_eq!(loaded, new_profile);

        Ok(())
    }

    #[test]
    fn upsert_load() -> anyhow::Result<()> {
        let connection = test_connection();
        let mut notifier = StoreNotifier::noop();

        let profile = test_profile();

        profile.upsert(&connection, &mut notifier)?;
        let loaded = UserProfile::load(&connection, &profile.user_name)?.expect("profile exists");
        assert_eq!(loaded, profile);

        let mut new_profile = profile.clone();
        new_profile.set_display_name(Some("Alice In Wonderland".to_string().try_into()?));
        new_profile.set_profile_picture(None);

        new_profile.upsert(&connection, &mut notifier)?;
        let loaded = UserProfile::load(&connection, &profile.user_name)?.expect("profile exists");
        assert_ne!(loaded, profile);
        assert_eq!(loaded, new_profile);

        Ok(())
    }

    #[test]
    fn update_load() -> anyhow::Result<()> {
        let connection = test_connection();
        let mut notifier = StoreNotifier::noop();

        let profile = test_profile();

        profile.store(&connection, &mut notifier)?;
        let loaded = UserProfile::load(&connection, &profile.user_name)?.expect("profile exists");
        assert_eq!(loaded, profile);

        let mut new_profile = profile.clone();
        new_profile.set_display_name(Some("Alice In Wonderland".to_string().try_into()?));
        new_profile.set_profile_picture(None);

        new_profile.update(&connection, &mut notifier)?;
        let loaded = UserProfile::load(&connection, &profile.user_name)?.expect("profile exists");
        assert_ne!(loaded, profile);
        assert_eq!(loaded, new_profile);

        Ok(())
    }
}
