// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use update_key_flow::UpdateKeyData;

use crate::{ConversationId, ConversationMessage};

use super::CoreUser;

impl CoreUser {
    /// Update the user's key material in the conversation with the given
    /// [`ConversationId`].
    ///
    /// Since this function causes the creation of an MLS commit, it can cause
    /// more than one effect on the group. As a result this function returns a
    /// vector of [`ConversationMessage`]s that represents the changes to the
    /// group. Note that these returned message have already been persisted.
    pub(crate) async fn update_key(
        &self,
        conversation_id: ConversationId,
    ) -> anyhow::Result<Vec<ConversationMessage>> {
        // Phase 1: Load the conversation and the group
        let updated = UpdateKeyData::load(self.pool(), conversation_id)
            .await?
            // Phase 2: Send the update to the DS
            .ds_update(&self.inner.api_clients)
            .await?;

        // Phase 3: Merge the commit into the group
        self.with_transaction_and_notifier(async |connection, notifier| {
            updated
                .merge_pending_commit(connection, notifier, conversation_id)
                .await
        })
        .await
    }
}

mod update_key_flow {
    use anyhow::Context;
    use phnxtypes::{messages::client_ds_out::UpdateParamsOut, time::TimeStamp};
    use sqlx::SqlitePool;

    use crate::{
        Conversation, ConversationId, ConversationMessage,
        clients::{CoreUser, api_clients::ApiClients},
        groups::Group,
    };

    pub(super) struct UpdateKeyData {
        conversation: Conversation,
        group: Group,
        params: UpdateParamsOut,
    }

    impl UpdateKeyData {
        pub(super) async fn load(
            pool: &SqlitePool,
            conversation_id: ConversationId,
        ) -> anyhow::Result<Self> {
            let conversation = Conversation::load(pool, &conversation_id)
                .await?
                .with_context(|| format!("Can't find conversation with id {conversation_id}"))?;
            let group_id = conversation.group_id();
            let mut group = Group::load(pool.acquire().await?.as_mut(), group_id)
                .await?
                .with_context(|| format!("Can't find group with id {group_id:?}"))?;
            let params = group.update(pool).await?;
            Ok(Self {
                conversation,
                group,
                params,
            })
        }

        pub(super) async fn ds_update(
            self,
            api_clients: &ApiClients,
        ) -> anyhow::Result<UpdatedKey> {
            let Self {
                conversation,
                group,
                params,
            } = self;
            let owner_domain = conversation.owner_domain();
            let ds_timestamp = api_clients
                .get(&owner_domain)?
                .ds_update(params, group.leaf_signer(), group.group_state_ear_key())
                .await?;
            Ok(UpdatedKey {
                group,
                ds_timestamp,
            })
        }
    }

    pub(super) struct UpdatedKey {
        group: Group,
        ds_timestamp: TimeStamp,
    }
    impl UpdatedKey {
        pub(crate) async fn merge_pending_commit(
            self,
            connection: &mut sqlx::SqliteConnection,
            notifier: &mut crate::store::StoreNotifier,
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
