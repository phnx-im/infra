// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::{Result, anyhow};
use create_conversation_flow::IntitialConversationData;
use delete_conversation_flow::load_conversation_data;
use tokio_util::either::Either;

use crate::{
    ConversationMessageId,
    conversations::{Conversation, messages::ConversationMessage},
    groups::openmls_provider::PhnxOpenMlsProvider,
};

use super::{ConversationId, CoreUser};

impl CoreUser {
    /// Create new conversation.
    ///
    /// Returns the id of the newly created conversation.
    pub(crate) async fn create_conversation(
        &self,
        title: String,
        picture: Option<Vec<u8>>,
    ) -> Result<ConversationId> {
        let group_data = IntitialConversationData::new(title, picture)
            .request_group_id(&self.inner.api_clients)
            .await?;

        let created_group = self
            .with_transaction_and_notifier(async |connection, notifier| {
                group_data
                    .create_group(
                        &PhnxOpenMlsProvider::new(&mut *connection),
                        &self.inner.key_store.signing_key,
                        &self.inner.key_store.connection_key,
                    )?
                    .store_group(&mut *connection, notifier)
                    .await
            })
            .await?;

        created_group
            .create_group_on_ds(&self.inner.api_clients, self.create_own_client_reference())
            .await
    }

    /// Delete the conversation with the given [`ConversationId`].
    ///
    /// Since this function causes the creation of an MLS commit, it can cause
    /// more than one effect on the group. As a result this function returns a
    /// vector of [`ConversationMessage`]s that represents the changes to the
    /// group. Note that these returned message have already been persisted.
    pub(crate) async fn delete_conversation(
        &self,
        conversation_id: ConversationId,
    ) -> Result<Vec<ConversationMessage>> {
        // Phase 1: Load the conversation and the group
        match load_conversation_data(self.pool(), conversation_id).await? {
            Either::Left(multi_user_data) => {
                let deleted = multi_user_data
                    // Phase 2: Create the delete commit
                    .create_delete_commit(self.pool())
                    .await?
                    // Phase 3: Send the delete to the DS
                    .send_delete_commit(&self.inner.api_clients)
                    .await?;
                self.with_transaction_and_notifier(async |connection, notifier| {
                    deleted
                        // Phase 4: Merge the commit into the group
                        .merge_pending_commit(&mut *connection)
                        .await?
                        // Phase 5: Set the conversation to inactive
                        .set_inactive(&mut *connection, notifier)
                        .await
                })
                .await
            }
            Either::Right(single_user_data) => {
                // Phase 5: Set the conversation to inactive
                self.with_transaction_and_notifier(async |connection, notifier| {
                    single_user_data.set_inactive(connection, notifier).await?;
                    Ok(Vec::new())
                })
                .await
            }
        }
    }

    pub(crate) async fn set_conversation_picture(
        &self,
        conversation_id: ConversationId,
        picture: Option<Vec<u8>>,
    ) -> Result<()> {
        let mut notifier = self.store_notifier();
        let mut conversation = Conversation::load(self.pool(), &conversation_id)
            .await?
            .ok_or_else(|| {
                let id = conversation_id.uuid();
                anyhow!("Can't find conversation with id {id}")
            })?;
        let resized_picture_option = picture.and_then(|picture| self.resize_image(&picture).ok());
        conversation
            .set_conversation_picture(self.pool(), &mut notifier, resized_picture_option)
            .await?;
        notifier.notify();
        Ok(())
    }

    pub(crate) async fn message(
        &self,
        message_id: ConversationMessageId,
    ) -> sqlx::Result<Option<ConversationMessage>> {
        ConversationMessage::load(self.pool(), message_id).await
    }

    pub(crate) async fn prev_message(
        &self,
        message_id: ConversationMessageId,
    ) -> Result<Option<ConversationMessage>> {
        Ok(ConversationMessage::prev_message(self.pool(), message_id).await?)
    }

    pub(crate) async fn next_message(
        &self,
        message_id: ConversationMessageId,
    ) -> Result<Option<ConversationMessage>> {
        Ok(ConversationMessage::next_message(self.pool(), message_id).await?)
    }

    pub(crate) async fn try_last_message(
        &self,
        conversation_id: ConversationId,
    ) -> sqlx::Result<Option<ConversationMessage>> {
        ConversationMessage::last_content_message(self.pool(), conversation_id).await
    }

    pub(crate) async fn conversations(&self) -> sqlx::Result<Vec<Conversation>> {
        Conversation::load_all(self.pool()).await
    }

    pub async fn conversation(&self, conversation_id: &ConversationId) -> Option<Conversation> {
        Conversation::load(self.pool(), conversation_id)
            .await
            .ok()
            .flatten()
    }

    /// Get the most recent `number_of_messages` messages from the conversation
    /// with the given [`ConversationId`].
    pub(crate) async fn get_messages(
        &self,
        conversation_id: ConversationId,
        number_of_messages: usize,
    ) -> Result<Vec<ConversationMessage>> {
        let messages = ConversationMessage::load_multiple(
            self.pool(),
            conversation_id,
            number_of_messages as u32,
        )
        .await?;
        Ok(messages)
    }
}

mod create_conversation_flow {
    use anyhow::Result;
    use openmls::group::GroupId;
    use openmls_traits::OpenMlsProvider;
    use phnxtypes::{
        codec::PhnxCodec, credentials::keys::ClientSigningKey, crypto::kdf::keys::ConnectionKey,
        identifiers::QsReference,
    };

    use crate::{
        Conversation, ConversationAttributes, ConversationId,
        clients::api_clients::ApiClients,
        groups::{Group, GroupData, PartialCreateGroupParams, client_auth_info::GroupMembership},
        store::StoreNotifier,
    };

    pub(super) struct IntitialConversationData {
        title: String,
        picture: Option<Vec<u8>>,
    }

    impl IntitialConversationData {
        pub(super) fn new(title: String, picture: Option<Vec<u8>>) -> Self {
            Self { title, picture }
        }

        pub(super) async fn request_group_id(
            self,
            api_clients: &ApiClients,
        ) -> Result<ConversationGroupData> {
            let Self { title, picture } = self;
            let group_id = api_clients.default_client()?.ds_request_group_id().await?;
            // Store the conversation attributes in the group's aad
            let attributes = ConversationAttributes::new(title, picture);
            let group_data = PhnxCodec::to_vec(&attributes)?.into();
            Ok(ConversationGroupData {
                group_id,
                group_data,
                attributes,
            })
        }
    }

    pub(super) struct ConversationGroupData {
        group_id: GroupId,
        group_data: GroupData,
        attributes: ConversationAttributes,
    }

    pub(super) struct CreatedGroup {
        group: Group,
        group_membership: GroupMembership,
        partial_params: PartialCreateGroupParams,
        attributes: ConversationAttributes,
    }

    impl ConversationGroupData {
        pub(super) fn create_group(
            self,
            provider: &impl OpenMlsProvider,
            signing_key: &ClientSigningKey,
            connection_key: &ConnectionKey,
        ) -> Result<CreatedGroup> {
            let Self {
                group_id,
                group_data,
                attributes,
            } = self;

            let (group, group_membership, partial_params) =
                Group::create_group(provider, signing_key, connection_key, group_id, group_data)?;

            Ok(CreatedGroup {
                group,
                group_membership,
                partial_params,
                attributes,
            })
        }
    }

    impl CreatedGroup {
        pub(super) async fn store_group(
            self,
            connection: &mut sqlx::SqliteConnection,
            notifier: &mut StoreNotifier,
        ) -> Result<StoredGroup> {
            let Self {
                group,
                group_membership,
                partial_params,
                attributes,
            } = self;

            group_membership.store(&mut *connection).await?;
            group.store(&mut *connection).await?;

            let conversation =
                Conversation::new_group_conversation(partial_params.group_id.clone(), attributes);
            conversation.store(&mut *connection, notifier).await?;

            Ok(StoredGroup {
                group,
                partial_params,
                conversation_id: conversation.id(),
            })
        }
    }

    pub(super) struct StoredGroup {
        group: Group,
        partial_params: PartialCreateGroupParams,
        conversation_id: ConversationId,
    }

    impl StoredGroup {
        pub(super) async fn create_group_on_ds(
            self,
            api_clients: &ApiClients,
            client_reference: QsReference,
        ) -> Result<ConversationId> {
            let Self {
                group,
                partial_params,
                conversation_id,
            } = self;

            let params = partial_params.into_params(client_reference);
            api_clients
                .default_client()?
                .ds_create_group(params, group.leaf_signer(), group.group_state_ear_key())
                .await?;

            Ok(conversation_id)
        }
    }
}

mod delete_conversation_flow {
    use std::collections::HashSet;

    use anyhow::Context;
    use phnxtypes::{
        identifiers::QualifiedUserName, messages::client_ds_out::DeleteGroupParamsOut,
        time::TimeStamp,
    };
    use sqlx::{SqliteConnection, SqlitePool};
    use tokio_util::either::Either;

    use crate::{
        Conversation, ConversationId, ConversationMessage, clients::api_clients::ApiClients,
        conversations::messages::TimestampedMessage, groups::Group, store::StoreNotifier,
    };

    pub(super) async fn load_conversation_data(
        pool: &SqlitePool,
        conversation_id: ConversationId,
    ) -> anyhow::Result<Either<LoadedConversationData<()>, LoadedSingleUserConversationData>> {
        let conversation = Conversation::load(pool, &conversation_id)
            .await?
            .with_context(|| format!("Can't find conversation with id {conversation_id}"))?;
        let group_id = conversation.group_id();
        let group = Group::load(pool.acquire().await?.as_mut(), group_id)
            .await?
            .with_context(|| format!("Can't find group with id {group_id:?}"))?;
        let past_members = group.members(pool).await;

        if past_members.len() == 1 {
            let member = past_members.into_iter().next().unwrap();
            Ok(Either::Right(LoadedSingleUserConversationData {
                conversation,
                member,
            }))
        } else {
            Ok(Either::Left(LoadedConversationData {
                conversation,
                group,
                past_members,
                state: (),
            }))
        }
    }

    pub(super) struct LoadedSingleUserConversationData {
        conversation: Conversation,
        member: QualifiedUserName,
    }

    impl LoadedSingleUserConversationData {
        pub(super) async fn set_inactive(
            self,
            connection: &mut SqliteConnection,
            notifier: &mut StoreNotifier,
        ) -> anyhow::Result<()> {
            let Self {
                mut conversation,
                member,
            } = self;
            conversation
                .set_inactive(connection, notifier, vec![member])
                .await?;
            Ok(())
        }
    }

    pub(super) struct LoadedConversationData<S> {
        conversation: Conversation,
        group: Group,
        past_members: HashSet<QualifiedUserName>,
        state: S,
    }

    impl LoadedConversationData<()> {
        pub(super) async fn create_delete_commit(
            self,
            pool: &SqlitePool,
        ) -> anyhow::Result<LoadedConversationData<DeleteGroupParamsOut>> {
            let Self {
                conversation,
                mut group,
                past_members,
                state: _,
            } = self;
            let params = group.delete(pool.acquire().await?.as_mut()).await?;
            Ok(LoadedConversationData {
                conversation,
                group,
                past_members,
                state: params,
            })
        }
    }

    impl LoadedConversationData<DeleteGroupParamsOut> {
        pub(super) async fn send_delete_commit(
            self,
            api_clients: &ApiClients,
        ) -> anyhow::Result<LoadedConversationData<DeletedGroupOnDs>> {
            let Self {
                conversation,
                group,
                past_members,
                state: params,
            } = self;
            let owner_domain = conversation.owner_domain();
            let ds_timestamp = api_clients
                .get(&owner_domain)?
                .ds_delete_group(params, group.leaf_signer(), group.group_state_ear_key())
                .await?;
            Ok(LoadedConversationData {
                conversation,
                group,
                past_members,
                state: DeletedGroupOnDs(ds_timestamp),
            })
        }
    }

    pub(super) struct DeletedGroupOnDs(TimeStamp);

    impl LoadedConversationData<DeletedGroupOnDs> {
        pub(super) async fn merge_pending_commit(
            self,
            connection: &mut SqliteConnection,
        ) -> anyhow::Result<DeletedGroup> {
            let Self {
                conversation,
                mut group,
                past_members,
                state: DeletedGroupOnDs(ds_timestamp),
            } = self;

            let messages = group
                .merge_pending_commit(connection, None, ds_timestamp)
                .await?;

            Ok(DeletedGroup {
                conversation,
                past_members,
                messages,
            })
        }
    }

    pub(super) struct DeletedGroup {
        conversation: Conversation,
        past_members: HashSet<QualifiedUserName>,
        messages: Vec<TimestampedMessage>,
    }

    impl DeletedGroup {
        pub(super) async fn set_inactive(
            self,
            connection: &mut SqliteConnection,
            notifier: &mut StoreNotifier,
        ) -> anyhow::Result<Vec<ConversationMessage>> {
            let Self {
                mut conversation,
                past_members,
                messages,
            } = self;
            conversation
                .set_inactive(
                    &mut *connection,
                    notifier,
                    past_members.into_iter().collect(),
                )
                .await?;

            let mut stored_messages = Vec::with_capacity(messages.len());
            for message in messages {
                let message =
                    ConversationMessage::from_timestamped_message(conversation.id(), message);
                message.store(&mut *connection, notifier).await?;
                stored_messages.push(message);
            }

            Ok(stored_messages)
        }
    }
}
