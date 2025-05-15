// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::identifiers::AsClientId;
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
        target_users: Vec<AsClientId>,
    ) -> anyhow::Result<Vec<ConversationMessage>> {
        // Phase 1: Load the group and conversation and prepare the commit.
        let removed = RemoveUsersData::load(self.pool(), conversation_id, target_users)
            .await?
            // Phase 2: Send the commit to the DS
            .ds_group_operation(&self.inner.api_clients)
            .await?;
        // Phase 3: Merge the commit into the group
        self.with_transaction_and_notifier(async |connection, notifier| {
            removed
                .merge_pending_commit(connection, notifier, conversation_id)
                .await
        })
        .await
    }
}

mod remove_users_flow {
    use anyhow::Context;
    use phnxtypes::{
        identifiers::AsClientId, messages::client_ds_out::GroupOperationParamsOut, time::TimeStamp,
    };
    use sqlx::SqlitePool;

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
        pub(super) async fn load(
            pool: &SqlitePool,
            conversation_id: ConversationId,
            target_users: Vec<AsClientId>,
        ) -> anyhow::Result<Self> {
            let mut connection = pool.acquire().await?;
            let conversation = Conversation::load(&mut connection, &conversation_id)
                .await?
                .with_context(|| format!("Can't find conversation with id {conversation_id}"))?;
            let group_id = conversation.group_id();
            let mut group = Group::load(&mut connection, group_id)
                .await?
                .with_context(|| format!("Can't find group with id {group_id:?}"))?;

            let params = group.remove(&mut connection, target_users).await?;
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
        pub(super) async fn merge_pending_commit(
            self,
            connection: &mut sqlx::SqliteConnection,
            notifier: &mut StoreNotifier,
            conversation_id: ConversationId,
        ) -> anyhow::Result<Vec<ConversationMessage>> {
            let Self {
                mut group,
                ds_timestamp,
            } = self;
            let group_messages = group
                .merge_pending_commit(&mut *connection, None, ds_timestamp)
                .await?;
            group.store_update(&mut *connection).await?;
            CoreUser::store_messages(&mut *connection, notifier, conversation_id, group_messages)
                .await
        }
    }
}
