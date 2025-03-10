// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::{Result, anyhow};
use openmls::prelude::GroupId;
use openmls_traits::OpenMlsProvider;
use phnxtypes::{
    codec::PhnxCodec, credentials::keys::ClientSigningKey, crypto::kdf::keys::ConnectionKey,
    identifiers::QsReference,
};
use tracing::error;

use crate::{
    ConversationMessageId,
    conversations::{Conversation, ConversationAttributes, messages::ConversationMessage},
    groups::{
        Group, GroupData, PartialCreateGroupParams, client_auth_info::GroupMembership,
        openmls_provider::PhnxOpenMlsProvider,
    },
};

use super::{ApiClients, ConversationId, CoreUser, StoreNotifier};

impl CoreUser {
    /// Create new conversation.
    ///
    /// Returns the id of the newly created conversation.
    pub async fn create_conversation(
        &self,
        title: String,
        picture: Option<Vec<u8>>,
    ) -> Result<ConversationId> {
        let group_data = IntitialConversationData { title, picture }
            .request_group_id(&self.inner.api_clients)
            .await?;

        let created_group = self
            .with_transaction(async move |transaction| {
                let provider = PhnxOpenMlsProvider::new(&mut *transaction);
                let mut notifier = self.store_notifier();

                let created_group = group_data
                    .create_group(
                        &provider,
                        &self.inner.key_store.signing_key,
                        &self.inner.key_store.connection_key,
                    )?
                    .store_group(&mut *transaction, &mut notifier)
                    .await?;

                notifier.notify();
                Ok(created_group)
            })
            .await?;

        created_group
            .create_group_on_ds(&self.inner.api_clients, self.create_own_client_reference())
            .await
    }

    pub async fn set_conversation_picture(
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

    pub async fn last_message(
        &self,
        conversation_id: ConversationId,
    ) -> Option<ConversationMessage> {
        ConversationMessage::last_content_message(self.pool(), conversation_id)
            .await
            .unwrap_or_else(|error| {
                error!(%error, "Error while fetching last message");
                None
            })
    }

    pub(crate) async fn try_last_message(
        &self,
        conversation_id: ConversationId,
    ) -> sqlx::Result<Option<ConversationMessage>> {
        ConversationMessage::last_content_message(self.pool(), conversation_id).await
    }

    pub async fn conversations(&self) -> sqlx::Result<Vec<Conversation>> {
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
    pub async fn get_messages(
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

struct IntitialConversationData {
    title: String,
    picture: Option<Vec<u8>>,
}

impl IntitialConversationData {
    async fn request_group_id(self, api_clients: &ApiClients) -> Result<ConversationGroupData> {
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

struct ConversationGroupData {
    group_id: GroupId,
    group_data: GroupData,
    attributes: ConversationAttributes,
}

struct CreatedGroup {
    group: Group,
    group_membership: GroupMembership,
    partial_params: PartialCreateGroupParams,
    attributes: ConversationAttributes,
}

impl ConversationGroupData {
    fn create_group(
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
    async fn store_group(
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

pub struct StoredGroup {
    group: Group,
    partial_params: PartialCreateGroupParams,
    conversation_id: ConversationId,
}

impl StoredGroup {
    async fn create_group_on_ds(
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
