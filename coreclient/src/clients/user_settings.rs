pub(crate) struct UserSettingRecord {}

mod persistence {
    use sqlx::SqliteExecutor;

    use super::UserSettingRecord;

    impl UserSettingRecord {
        pub(crate) async fn load(
            executor: impl SqliteExecutor<'_>,
            setting: &'static str,
        ) -> sqlx::Result<Option<Vec<u8>>> {
            sqlx::query_scalar!("SELECT value FROM user_settings WHERE setting = ?", setting)
                .fetch_optional(executor)
                .await
        }

        pub(crate) async fn store(
            executor: impl SqliteExecutor<'_>,
            setting: &str,
            value: Vec<u8>,
        ) -> sqlx::Result<()> {
            sqlx::query!(
                "INSERT OR REPLACE INTO user_settings (setting, value) VALUES (?, ?)",
                setting,
                value
            )
            .execute(executor)
            .await?;
            Ok(())
        }
    }
}
