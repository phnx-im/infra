// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::{anyhow, Result};
use openmls::prelude::GroupId;
use openmls_traits::OpenMlsProvider;
use phnxtypes::{
    codec::PhnxCodec, credentials::keys::ClientSigningKey, crypto::ear::EarEncryptable,
    identifiers::QsClientReference,
};
use rusqlite::Connection;

use crate::{
    conversations::{messages::ConversationMessage, Conversation, ConversationAttributes},
    groups::{
        client_auth_info::GroupMembership, openmls_provider::PhnxOpenMlsProvider, Group, GroupData,
        PartialCreateGroupParams,
    },
    ConversationMessageId,
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

        let created_group = {
            let mut connection = self.inner.connection.lock().await;
            let transaction = connection.transaction()?;

            let provider = PhnxOpenMlsProvider::new(&transaction);
            let mut notifier = self.store_notifier();

            let created_group = group_data
                .create_group(&provider, &self.inner.key_store.signing_key)?
                .store_group(&transaction, &mut notifier)?;

            transaction.commit()?;
            notifier.notify();

            created_group
        };

        created_group
            .create_group_on_ds(
                &self.inner.api_clients,
                &self.inner.key_store.signing_key,
                self.create_own_client_reference(),
            )
            .await
    }

    pub async fn set_conversation_picture(
        &self,
        conversation_id: ConversationId,
        picture: Option<Vec<u8>>,
    ) -> Result<()> {
        let connection = &self.inner.connection.lock().await;
        let mut notifier = self.store_notifier();
        let mut conversation =
            Conversation::load(connection, &conversation_id)?.ok_or_else(|| {
                let id = conversation_id.as_uuid();
                anyhow!("Can't find conversation with id {id}")
            })?;
        let resized_picture_option = picture.and_then(|picture| self.resize_image(&picture).ok());
        conversation.set_conversation_picture(connection, &mut notifier, resized_picture_option)?;
        notifier.notify();
        Ok(())
    }

    pub(crate) async fn message(
        &self,
        message_id: ConversationMessageId,
    ) -> Result<Option<ConversationMessage>, rusqlite::Error> {
        let connection = &self.inner.connection.lock().await;
        ConversationMessage::load(connection, &message_id.to_uuid())
    }

    pub(crate) async fn try_last_message(
        &self,
        conversation_id: ConversationId,
    ) -> Result<Option<ConversationMessage>, rusqlite::Error> {
        let connection = &self.inner.connection.lock().await;
        ConversationMessage::last_content_message(connection, conversation_id)
    }

    pub async fn conversations(&self) -> Result<Vec<Conversation>, rusqlite::Error> {
        let connection = &self.inner.connection.lock().await;
        let conversations = Conversation::load_all(connection)?;
        Ok(conversations)
    }

    pub async fn conversation(&self, conversation_id: &ConversationId) -> Option<Conversation> {
        let connection = self.inner.connection.lock().await;
        Conversation::load(&connection, conversation_id)
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
        let connection = self.inner.connection.lock().await;
        let messages = ConversationMessage::load_multiple(
            &connection,
            conversation_id,
            number_of_messages as u32,
        )?;
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
    fn store_group(
        self,
        connection: &Connection,
        notifier: &mut StoreNotifier,
    ) -> Result<StoredGroup> {
        let Self {
            group,
            group_membership,
            partial_params,
            attributes,
        } = self;

        group_membership.store(connection)?;
        group.store(connection)?;

        let conversation =
            Conversation::new_group_conversation(partial_params.group_id.clone(), attributes);
        conversation.store(connection, notifier)?;

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
        signing_key: &ClientSigningKey,
        client_reference: QsClientReference,
    ) -> Result<ConversationId> {
        let Self {
            group,
            partial_params,
            conversation_id,
        } = self;

        let encrypted_client_credential = signing_key
            .credential()
            .encrypt(group.credential_ear_key())?;
        let params = partial_params.into_params(encrypted_client_credential, client_reference);
        api_clients
            .default_client()?
            .ds_create_group(
                params,
                group.group_state_ear_key(),
                group
                    .user_auth_key()
                    .ok_or_else(|| anyhow!("No user auth key"))?,
            )
            .await?;

        Ok(conversation_id)
    }
}
