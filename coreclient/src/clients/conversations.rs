// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use aircommon::identifiers::UserId;
use anyhow::{Result, anyhow, bail};
use create_conversation_flow::IntitialConversationData;
use delete_conversation_flow::DeleteConversationData;
use leave_conversation_flow::LeaveConversationData;
use mimi_room_policy::VerifiedRoomState;

use crate::{
    ConversationMessageId,
    conversations::{Conversation, messages::ConversationMessage},
    groups::{Group, openmls_provider::AirOpenMlsProvider},
    utils::image::resize_profile_image,
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
                        &AirOpenMlsProvider::new(&mut *connection),
                        &self.inner.key_store.signing_key,
                    )?
                    .store_group(&mut *connection, notifier)
                    .await
            })
            .await?;

        created_group
            .create_group_on_ds(
                &self.inner.api_clients,
                self.signing_key(),
                self.create_own_client_reference(),
            )
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
        let mut txn = self.pool().begin_with("BEGIN IMMEDIATE").await?;

        let delete_conversation_data =
            DeleteConversationData::load(&mut txn, conversation_id).await?;

        match delete_conversation_data {
            DeleteConversationData::SingleMember(data) => {
                // No need to send a message to the server if we are the only member.
                // Phase 5: Set the conversation to inactive
                self.with_notifier(async |notifier| data.set_inactive(&mut txn, notifier).await)
                    .await?;
                txn.commit().await?;

                Ok(Vec::new())
            }
            DeleteConversationData::MultiMember(data) => {
                // Phase 2: Create the delete commit
                let delete = data
                    .stage_delete_commit(&mut txn, self.signing_key())
                    .await?;
                txn.commit().await?;

                // Phase 3: Send the delete to the DS
                let deleted = delete
                    .send_delete_commit(&self.inner.api_clients, self.signing_key())
                    .await?;
                // TODO: Retry send until we get a response
                self.with_transaction_and_notifier(async |connection, notifier| {
                    deleted
                        // Phase 4: Merge the commit into the group
                        .merge_pending_commit(&mut *connection)
                        .await?
                        // Phase 5: Set the conversation to inactive
                        .set_inactive(&mut *connection, notifier, conversation_id)
                        .await
                })
                .await
            }
        }
    }

    pub(crate) async fn leave_conversation(&self, conversation_id: ConversationId) -> Result<()> {
        let leave = self
            .with_transaction(async |txn| {
                // Phase 1: Load the conversation and the group
                LeaveConversationData::load(txn, conversation_id)
                    .await?
                    .stage_leave_group(self.user_id(), txn, self.signing_key())
                    .await
            })
            .await?;

        // Phase 2: Send the leave to the DS
        leave
            .ds_self_remove(&self.inner.api_clients, self.signing_key())
            .await?
            // Phase 3: Merge the commit into the group
            .store_update(self.pool())
            .await?;
        Ok(())
    }

    pub(crate) async fn set_conversation_picture(
        &self,
        conversation_id: ConversationId,
        picture: Option<Vec<u8>>,
    ) -> Result<()> {
        let mut connection = self.pool().acquire().await?;
        let mut conversation = Conversation::load(&mut connection, &conversation_id)
            .await?
            .ok_or_else(|| {
                let id = conversation_id.uuid();
                anyhow!("Can't find conversation with id {id}")
            })?;
        let resized_picture_option =
            picture.and_then(|picture| resize_profile_image(&picture).ok());
        let mut notifier = self.store_notifier();
        conversation
            .set_conversation_picture(&mut *connection, &mut notifier, resized_picture_option)
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

    pub(crate) async fn conversations(&self) -> sqlx::Result<Vec<Conversation>> {
        Conversation::load_all(self.pool().acquire().await?.as_mut()).await
    }

    pub async fn conversation(&self, conversation_id: &ConversationId) -> Option<Conversation> {
        Conversation::load(self.pool().acquire().await.ok()?.as_mut(), conversation_id)
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

    pub async fn load_room_state(
        &self,
        conversation_id: &ConversationId,
    ) -> Result<(UserId, VerifiedRoomState)> {
        if let Some(conversation) = self.conversation(conversation_id).await {
            let mut connection = self.pool().acquire().await?;
            if let Some(group) = Group::load(&mut connection, conversation.group_id()).await? {
                return Ok((self.user_id().clone(), group.room_state));
            }
        }
        bail!("Room does not exist")
    }
}

mod create_conversation_flow {
    use aircommon::{
        codec::PersistenceCodec,
        credentials::keys::ClientSigningKey,
        crypto::{ear::keys::EncryptedUserProfileKey, indexed_aead::keys::UserProfileKey},
        identifiers::QsReference,
    };
    use anyhow::Result;
    use openmls::group::GroupId;
    use openmls_traits::OpenMlsProvider;

    use crate::{
        Conversation, ConversationAttributes, ConversationId,
        clients::api_clients::ApiClients,
        groups::{Group, GroupData, PartialCreateGroupParams, client_auth_info::GroupMembership},
        key_stores::indexed_keys::StorableIndexedKey,
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
            let group_data = PersistenceCodec::to_vec(&attributes)?.into();
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
        ) -> Result<CreatedGroup> {
            let Self {
                group_id,
                group_data,
                attributes,
            } = self;

            let (group, group_membership, partial_params) =
                Group::create_group(provider, signing_key, group_id, group_data)?;

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
            txn: &mut sqlx::SqliteTransaction<'_>,
            notifier: &mut StoreNotifier,
        ) -> Result<StoredGroup> {
            let Self {
                group,
                group_membership,
                partial_params,
                attributes,
            } = self;

            let user_profile_key = UserProfileKey::load_own(txn.as_mut()).await?;
            let encrypted_user_profile_key = user_profile_key.encrypt(
                group.identity_link_wrapper_key(),
                group_membership.user_id(),
            )?;

            group_membership.store(txn.as_mut()).await?;
            group.store(txn.as_mut()).await?;

            let conversation =
                Conversation::new_group_conversation(partial_params.group_id.clone(), attributes);
            conversation.store(txn.as_mut(), notifier).await?;

            Ok(StoredGroup {
                group,
                encrypted_user_profile_key,
                partial_params,
                conversation_id: conversation.id(),
            })
        }
    }

    pub(super) struct StoredGroup {
        group: Group,
        encrypted_user_profile_key: EncryptedUserProfileKey,
        partial_params: PartialCreateGroupParams,
        conversation_id: ConversationId,
    }

    impl StoredGroup {
        pub(super) async fn create_group_on_ds(
            self,
            api_clients: &ApiClients,
            signer: &ClientSigningKey,
            client_reference: QsReference,
        ) -> Result<ConversationId> {
            let Self {
                group,
                encrypted_user_profile_key,
                partial_params,
                conversation_id,
            } = self;

            let params = partial_params.into_params(client_reference, encrypted_user_profile_key);
            api_clients
                .default_client()?
                .ds_create_group(params, signer, group.group_state_ear_key())
                .await?;

            Ok(conversation_id)
        }
    }
}

mod delete_conversation_flow {
    use std::collections::HashSet;

    use aircommon::{
        credentials::keys::ClientSigningKey, identifiers::UserId,
        messages::client_ds_out::DeleteGroupParamsOut, time::TimeStamp,
    };
    use anyhow::Context;
    use sqlx::{SqliteConnection, SqliteTransaction};

    use crate::{
        Conversation, ConversationId, ConversationMessage,
        clients::{CoreUser, api_clients::ApiClients},
        conversations::messages::TimestampedMessage,
        groups::Group,
        store::StoreNotifier,
    };

    pub(super) enum DeleteConversationData {
        SingleMember(Box<LoadedSingleUserConversationData>),
        MultiMember(Box<LoadedConversationData<()>>),
    }

    impl DeleteConversationData {
        pub(super) async fn load(
            txn: &mut SqliteTransaction<'_>,
            conversation_id: ConversationId,
        ) -> anyhow::Result<Self> {
            let conversation = Conversation::load(txn.as_mut(), &conversation_id)
                .await?
                .with_context(|| format!("Can't find conversation with id {conversation_id}"))?;

            let group_id = conversation.group_id();
            let group = Group::load_clean(txn, group_id)
                .await?
                .with_context(|| format!("Can't find group with id {group_id:?}"))?;

            let past_members = group.members(txn.as_mut()).await;

            if past_members.len() == 1 {
                let member = past_members.into_iter().next().unwrap();
                Ok(Self::SingleMember(
                    LoadedSingleUserConversationData {
                        conversation,
                        member,
                    }
                    .into(),
                ))
            } else {
                Ok(Self::MultiMember(
                    LoadedConversationData {
                        conversation,
                        group,
                        past_members,
                        state: (),
                    }
                    .into(),
                ))
            }
        }
    }

    pub(super) struct LoadedSingleUserConversationData {
        conversation: Conversation,
        member: UserId,
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
        past_members: HashSet<UserId>,
        state: S,
    }

    impl LoadedConversationData<()> {
        pub(super) async fn stage_delete_commit(
            self,
            connection: &mut SqliteConnection,
            signer: &ClientSigningKey,
        ) -> anyhow::Result<LoadedConversationData<DeleteGroupParamsOut>> {
            let Self {
                conversation,
                mut group,
                past_members,
                state: _,
            } = self;
            let params = group.stage_delete(connection, signer).await?;
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
            signer: &ClientSigningKey,
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
                .ds_delete_group(params, signer, group.group_state_ear_key())
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
        past_members: HashSet<UserId>,
        messages: Vec<TimestampedMessage>,
    }

    impl DeletedGroup {
        pub(super) async fn set_inactive(
            self,
            connection: &mut SqliteConnection,
            notifier: &mut StoreNotifier,
            conversation_id: ConversationId,
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
            CoreUser::store_new_messages(&mut *connection, notifier, conversation_id, messages)
                .await
        }
    }
}

mod leave_conversation_flow {
    use aircommon::{
        credentials::keys::ClientSigningKey, identifiers::UserId,
        messages::client_ds_out::SelfRemoveParamsOut,
    };
    use anyhow::Context;
    use mimi_room_policy::RoleIndex;
    use sqlx::{SqliteConnection, SqlitePool, SqliteTransaction};

    use crate::{Conversation, ConversationId, groups::Group};

    pub(super) struct LeaveConversationData<S> {
        conversation: Conversation,
        group: Group,
        state: S,
    }

    impl LeaveConversationData<()> {
        pub(super) async fn load(
            txn: &mut SqliteTransaction<'_>,
            conversation_id: ConversationId,
        ) -> anyhow::Result<LeaveConversationData<()>> {
            let conversation = Conversation::load(txn.as_mut(), &conversation_id)
                .await?
                .with_context(|| format!("Can't find conversation with id {conversation_id}",))?;
            let group_id = conversation.group_id();
            let group = Group::load_clean(txn, group_id)
                .await?
                .with_context(|| format!("Can't find group with id {group_id:?}"))?;
            Ok(Self {
                conversation,
                group,
                state: (),
            })
        }

        pub(super) async fn stage_leave_group(
            self,
            sender_id: &UserId,
            connection: &mut SqliteConnection,
            signer: &ClientSigningKey,
        ) -> anyhow::Result<LeaveConversationData<SelfRemoveParamsOut>> {
            let Self {
                conversation,
                mut group,
                state: (),
            } = self;

            group.room_state_change_role(sender_id, sender_id, RoleIndex::Outsider)?;

            let params = group.stage_leave_group(connection, signer)?;

            Ok(LeaveConversationData {
                conversation,
                group,
                state: params,
            })
        }
    }

    impl LeaveConversationData<SelfRemoveParamsOut> {
        pub(super) async fn ds_self_remove(
            self,
            api_clients: &crate::clients::api_clients::ApiClients,
            signer: &ClientSigningKey,
        ) -> anyhow::Result<DsSelfRemoved> {
            let Self {
                conversation,
                group,
                state: params,
            } = self;

            let owner_domain = conversation.owner_domain();

            api_clients
                .get(&owner_domain)?
                .ds_self_remove(params, signer, group.group_state_ear_key())
                .await?;

            Ok(DsSelfRemoved(group))
        }
    }

    pub(super) struct DsSelfRemoved(Group);

    impl DsSelfRemoved {
        pub(super) async fn store_update(self, pool: &SqlitePool) -> anyhow::Result<()> {
            let Self(group) = self;
            group.store_update(pool).await?;
            Ok(())
        }
    }
}
