// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use aircommon::identifiers::UserId;
use chrono::{DateTime, Utc};

use crate::{clients::CoreUser, user_profiles::display_name::BaseDisplayName};

impl CoreUser {
    pub(crate) async fn block_contact(&self, user_id: UserId) -> anyhow::Result<()> {
        let profile = self.user_profile(&user_id).await;
        let blocked_contact = BlockedContact {
            user_id,
            last_display_name: profile.display_name.clone(),
            blocked_at: Utc::now(),
        };
        self.with_notifier(async |notifier| {
            Ok(blocked_contact.store(self.pool(), notifier).await?)
        })
        .await
    }

    pub(crate) async fn unblock_contact(&self, user_id: UserId) -> anyhow::Result<()> {
        self.with_notifier(async |notifier| {
            Ok(BlockedContact::delete_by_id(self.pool(), notifier, user_id).await?)
        })
        .await
    }
}

struct BlockedContact {
    user_id: UserId,
    last_display_name: BaseDisplayName<true>,
    blocked_at: DateTime<Utc>,
}

mod persistence {
    use sqlx::{SqliteExecutor, query};

    use crate::store::StoreNotifier;

    use super::*;

    impl BlockedContact {
        pub(super) async fn store(
            &self,
            executor: impl SqliteExecutor<'_>,
            notifier: &mut StoreNotifier,
        ) -> sqlx::Result<()> {
            let uuid = self.user_id.uuid();
            let domain = self.user_id.domain();
            query!(
                "INSERT INTO blocked_contact (
                    user_uuid,
                    user_domain,
                    last_display_name,
                    blocked_at
                ) VALUES (?1, ?2, ?3, ?4)",
                uuid,
                domain,
                self.last_display_name,
                self.blocked_at,
            )
            .execute(executor)
            .await?;

            notifier.add(self.user_id.clone());

            Ok(())
        }

        pub(super) async fn delete_by_id(
            executor: impl SqliteExecutor<'_>,
            notifier: &mut StoreNotifier,
            user_id: UserId,
        ) -> sqlx::Result<()> {
            let uuid = user_id.uuid();
            let domain = user_id.domain();
            query!(
                "DELETE FROM blocked_contact WHERE user_uuid = ?1 AND user_domain = ?2",
                uuid,
                domain,
            )
            .execute(executor)
            .await?;

            notifier.add(user_id.clone());

            Ok(())
        }
    }
}
