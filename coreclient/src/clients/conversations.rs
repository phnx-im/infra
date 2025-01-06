// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::{anyhow, Result};
use phnxtypes::{codec::PhnxCodec, crypto::ear::EarEncryptable};

use crate::{
    conversations::{messages::ConversationMessage, Conversation, ConversationAttributes},
    groups::Group,
    ConversationMessageId,
};

use super::{ConversationId, CoreUser};

impl CoreUser {
    /// Create new conversation.
    ///
    /// Returns the id of the newly created conversation.
    pub async fn create_conversation(
        &self,
        title: &str,
        conversation_picture_option: Option<Vec<u8>>,
    ) -> Result<ConversationId> {
        let group_id = self
            .inner
            .api_clients
            .default_client()?
            .ds_request_group_id()
            .await?;
        let client_reference = self.create_own_client_reference();
        // Store the conversation attributes in the group's aad
        let conversation_attributes =
            ConversationAttributes::new(title.to_string(), conversation_picture_option);
        let group_data = PhnxCodec::to_vec(&conversation_attributes)?.into();

        // Phase 1: Create and store the group in the OpenMLS provider
        let mut connection = self.inner.connection.lock().await;
        let mut notifier = self.store_notifier();
        let (group, partial_params) = Group::create_group(
            &mut connection,
            &self.inner.key_store.signing_key,
            group_id.clone(),
            group_data,
        )?;
        group.store(&connection)?;
        let conversation = Conversation::new_group_conversation(group_id, conversation_attributes);
        conversation.store(&connection, &mut notifier)?;

        drop(connection);
        notifier.notify();

        // Phase 2: Create the group on the DS
        let encrypted_client_credential = self
            .inner
            .key_store
            .signing_key
            .credential()
            .encrypt(group.credential_ear_key())?;
        let params = partial_params.into_params(encrypted_client_credential, client_reference);
        self.inner
            .api_clients
            .default_client()?
            .ds_create_group(
                params,
                group.group_state_ear_key(),
                group.user_auth_key().ok_or(anyhow!("No user auth key"))?,
            )
            .await?;

        Ok(conversation.id())
    }

    pub async fn set_conversation_picture(
        &self,
        conversation_id: ConversationId,
        conversation_picture_option: Option<Vec<u8>>,
    ) -> Result<()> {
        let connection = &self.inner.connection.lock().await;
        let mut notifier = self.store_notifier();
        let mut conversation = Conversation::load(connection, &conversation_id)?.ok_or(anyhow!(
            "Can't find conversation with id {}",
            conversation_id.as_uuid()
        ))?;
        let resized_picture_option = conversation_picture_option
            .and_then(|conversation_picture| self.resize_image(&conversation_picture).ok());
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

    pub async fn last_message(
        &self,
        conversation_id: ConversationId,
    ) -> Option<ConversationMessage> {
        let connection = &self.inner.connection.lock().await;
        ConversationMessage::last_content_message(connection, conversation_id).unwrap_or_else(|e| {
            log::error!("Error while fetching last message: {:?}", e);
            None
        })
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
