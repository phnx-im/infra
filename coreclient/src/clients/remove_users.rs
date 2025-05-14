// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::identifiers::QualifiedUserName;
use remove_users_flow::RemoveUsersData;

use crate::{ConversationId, ConversationMessage};

use super::CoreUser;

impl CoreUser {
    /// Remove users from the conversation with the given [`ConversationId`].
    ///
    /// Since this function causes the creation of an MLS commit, it can cause
    /// more than one effect on the group. As a result this function returns a
    /// vector of [`ConversationMessage`]s that represents the changes to the
    /// group. Note that these returned message have already been persisted.
    pub(crate) async fn remove_users(
        &self,
        conversation_id: ConversationId,
        target_users: &[QualifiedUserName],
    ) -> anyhow::Result<Vec<ConversationMessage>> {
        // Phase 1: Load the group and conversation and prepare the commit.
        let remove = self
            .with_transaction(async |txn| {
                RemoveUsersData::stage_remove(txn, conversation_id, target_users).await
            })
            .await?;

        // Phase 2: Send the commit to the DS
        let removed = remove.ds_group_operation(&self.inner.api_clients).await?;

        // Phase 3: Merge the commit into the group
        self.with_transaction_and_notifier(async |txn, notifier| {
            removed.accept(txn, notifier, conversation_id).await
        })
        .await
    }
}

mod remove_users_flow {
    use anyhow::Context;
    use anyhow::anyhow;
    use phnxtypes::{
        identifiers::QualifiedUserName, messages::client_ds_out::GroupOperationParamsOut,
        time::TimeStamp,
    };
    use sqlx::SqliteTransaction;

    use crate::{
        Conversation, ConversationId, ConversationMessage,
        clients::{CoreUser, api_clients::ApiClients},
        groups::Group,
        store::StoreNotifier,
    };

    pub(super) struct RemoveUsersData {
        conversation: Conversation,
        group: Group,
        params: GroupOperationParamsOut,
    }

    impl RemoveUsersData {
        pub(super) async fn stage_remove(
            txn: &mut SqliteTransaction<'_>,
            conversation_id: ConversationId,
            target_users: &[QualifiedUserName],
        ) -> anyhow::Result<Self> {
            let conversation = Conversation::load(&mut **txn, &conversation_id)
                .await?
                .with_context(|| format!("Can't find conversation with id {conversation_id}"))?;
            let group_id = conversation.group_id();
            let mut group = Group::load_clean(txn, group_id)
                .await?
                .ok_or_else(|| anyhow!("No group found for group ID {:?}", group_id))?;

            let mut clients = Vec::with_capacity(target_users.len());

            for user_name in target_users {
                clients.extend(group.user_client_ids(&mut **txn, user_name).await);
            }

            let params = group.stage_remove(&mut *txn, clients).await?;

            Ok(Self {
                conversation,
                group,
                params,
            })
        }

        pub(super) async fn ds_group_operation(
            self,
            api_clients: &ApiClients,
        ) -> anyhow::Result<RemovedUsers> {
            let Self {
                conversation,
                group,
                params,
            } = self;

            let ds_timestamp = api_clients
                .get(&conversation.owner_domain())?
                .ds_group_operation(params, group.leaf_signer(), group.group_state_ear_key())
                .await?;
            Ok(RemovedUsers {
                group,
                ds_timestamp,
            })
        }
    }

    pub(super) struct RemovedUsers {
        group: Group,
        ds_timestamp: TimeStamp,
    }

    impl RemovedUsers {
        pub(super) async fn accept(
            self,
            txn: &mut sqlx::SqliteTransaction<'_>,
            notifier: &mut StoreNotifier,
            conversation_id: ConversationId,
        ) -> anyhow::Result<Vec<ConversationMessage>> {
            let Self {
                mut group,
                ds_timestamp,
            } = self;

            let group_messages = group
                .merge_pending_commit(&mut *txn, None, ds_timestamp)
                .await?;
            group.store_update(&mut **txn).await?;
            CoreUser::store_messages(&mut *txn, notifier, conversation_id, group_messages).await
        }
    }
}
