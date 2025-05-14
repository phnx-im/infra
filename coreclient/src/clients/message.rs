// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::{Context, bail};
use mimi_content::MimiContent;
use openmls::storage::OpenMlsProvider;
use phnxtypes::{
    identifiers::QualifiedUserName, messages::client_ds_out::SendMessageParamsOut, time::TimeStamp,
};
use sqlx::SqliteConnection;
use uuid::Uuid;

use crate::{Conversation, ConversationId, ConversationMessage, ConversationMessageId, Message};

use super::{ApiClients, CoreUser, Group, PhnxOpenMlsProvider, StoreNotifier};

impl CoreUser {
    /// Send a message and return it.
    ///
    /// The message unsent messages is stored, then sent to the DS and finally returned. The
    /// conversation is marked as read until this message.
    pub(crate) async fn send_message(
        &self,
        conversation_id: ConversationId,
        content: MimiContent,
    ) -> anyhow::Result<ConversationMessage> {
        let unsent_group_message = self
            .with_transaction_and_notifier(async |connection, notifier| {
                UnsentContent {
                    conversation_id,
                    content,
                }
                .store_unsent_message(connection, notifier, self.user_name())
                .await?
                .create_group_message(&PhnxOpenMlsProvider::new(connection))?
                .store_group_update(connection, notifier)
                .await
            })
            .await?;

        let sent_message = unsent_group_message
            .send_message_to_ds(&self.inner.api_clients)
            .await?;

        self.with_transaction_and_notifier(async |connection, notifier| {
            sent_message
                .mark_as_sent_and_read(connection, notifier)
                .await
        })
        .await
    }

    /// Re-try sending a message, where sending previously failed.
    pub async fn re_send_message(&self, local_message_id: Uuid) -> anyhow::Result<()> {
        let unsent_group_message = self
            .with_transaction(async |connection| {
                LocalMessage { local_message_id }
                    .load_for_resend(connection)
                    .await?
                    .create_group_message(&PhnxOpenMlsProvider::new(connection))
            })
            .await?;

        let sent_message = unsent_group_message
            .send_message_to_ds(&self.inner.api_clients)
            .await?;

        self.with_transaction_and_notifier(async |connection, notifier| {
            sent_message
                .mark_as_sent_and_read(connection, notifier)
                .await
        })
        .await?;

        Ok(())
    }
}

struct UnsentContent {
    conversation_id: ConversationId,
    content: MimiContent,
}

impl UnsentContent {
    async fn store_unsent_message(
        self,
        txn: &mut sqlx::SqliteTransaction<'_>,
        notifier: &mut StoreNotifier,
        sender: &QualifiedUserName,
    ) -> anyhow::Result<UnsentMessage<WithContent, GroupUpdateNeeded>> {
        let UnsentContent {
            conversation_id,
            content,
        } = self;

        let conversation = Conversation::load(txn.as_mut(), &conversation_id)
            .await?
            .with_context(|| format!("Can't find conversation with id {conversation_id}"))?;
        // Store the message as unsent so that we don't lose it in case
        // something goes wrong.
        let conversation_message = ConversationMessage::new_unsent_message(
            sender.to_string(),
            conversation_id,
            content.clone(),
        );
        conversation_message.store(txn.as_mut(), notifier).await?;

        let group_id = conversation.group_id();
        let group = Group::load_clean(txn, group_id)
            .await?
            .with_context(|| format!("Can't find group with id {group_id:?}"))?;

        Ok(UnsentMessage {
            conversation,
            group,
            conversation_message,
            content: WithContent(content),
            group_update: GroupUpdateNeeded,
        })
    }
}

struct LocalMessage {
    local_message_id: Uuid,
}

impl LocalMessage {
    async fn load_for_resend(
        self,
        connection: &mut SqliteConnection,
    ) -> anyhow::Result<UnsentMessage<WithContent, GroupUpdated>> {
        let Self { local_message_id } = self;

        let conversation_message = ConversationMessage::load(
            &mut *connection,
            ConversationMessageId::new(local_message_id),
        )
        .await?
        .with_context(|| format!("Can't find unsent message with id {local_message_id}"))?;
        let content = match conversation_message.message() {
            Message::Content(content_message) if !content_message.was_sent() => {
                content_message.content().clone()
            }
            Message::Content(_) => bail!("Message with id {local_message_id} was already sent"),
            _ => bail!("Message with id {local_message_id} is not a content message"),
        };
        let conversation_id = conversation_message.conversation_id();
        let conversation = Conversation::load(&mut *connection, &conversation_id)
            .await?
            .with_context(|| format!("Can't find conversation with id {conversation_id}"))?;
        let group_id = conversation.group_id();

        let group = Group::load(connection, group_id)
            .await?
            .with_context(|| format!("Can't find group with id {group_id:?}"))?;

        let message = UnsentMessage {
            conversation,
            group,
            conversation_message,
            content: WithContent(content),
            group_update: GroupUpdated,
        };

        Ok(message)
    }
}

/// Message type state: Message with MIMI content
struct WithContent(MimiContent);
/// Message type state: Message with prepared send parameters
struct WithParams(SendMessageParamsOut);

/// Message type state: Group update needed before sending the message
struct GroupUpdateNeeded;
/// Message type state: Group already updated, message can be sent
struct GroupUpdated;

struct UnsentMessage<State, GroupUpdate> {
    conversation: Conversation,
    group: Group,
    conversation_message: ConversationMessage,
    content: State,
    group_update: GroupUpdate,
}

impl<GroupUpdate> UnsentMessage<WithContent, GroupUpdate> {
    fn create_group_message(
        self,
        provider: &impl OpenMlsProvider,
    ) -> anyhow::Result<UnsentMessage<WithParams, GroupUpdate>> {
        let Self {
            conversation,
            mut group,
            conversation_message,
            content: WithContent(content),
            group_update,
        } = self;

        let params = group.create_message(provider, content)?;

        Ok(UnsentMessage {
            conversation,
            conversation_message,
            group,
            content: WithParams(params),
            group_update,
        })
    }
}

impl UnsentMessage<WithParams, GroupUpdateNeeded> {
    async fn store_group_update(
        self,
        connection: &mut sqlx::SqliteConnection,
        notifier: &mut StoreNotifier,
    ) -> anyhow::Result<UnsentMessage<WithParams, GroupUpdated>> {
        let Self {
            conversation,
            group,
            conversation_message,
            content: WithParams(params),
            group_update: GroupUpdateNeeded,
        } = self;

        // Immediately write the group back. No need to wait for the DS to
        // confirm as this is just an application message.
        group.store_update(&mut *connection).await?;
        // Also, mark the message (and all messages preceeding it) as read.
        Conversation::mark_as_read_until_message_id(
            connection,
            notifier,
            conversation.id(),
            conversation_message.id(),
        )
        .await?;

        Ok(UnsentMessage {
            conversation,
            group,
            conversation_message,
            content: WithParams(params),
            group_update: GroupUpdated,
        })
    }
}

impl UnsentMessage<WithParams, GroupUpdated> {
    async fn send_message_to_ds(self, api_clients: &ApiClients) -> anyhow::Result<SentMessage> {
        let Self {
            conversation,
            conversation_message,
            group,
            content: WithParams(params),
            group_update: GroupUpdated,
        } = self;

        let ds_timestamp = api_clients
            .get(&conversation.owner_domain())?
            .ds_send_message(params, group.leaf_signer(), group.group_state_ear_key())
            .await?;

        Ok(SentMessage {
            conversation_message,
            ds_timestamp,
        })
    }
}

struct SentMessage {
    conversation_message: ConversationMessage,
    ds_timestamp: TimeStamp,
}

impl SentMessage {
    async fn mark_as_sent_and_read(
        self,
        connection: &mut sqlx::SqliteConnection,
        notifier: &mut StoreNotifier,
    ) -> anyhow::Result<ConversationMessage> {
        let Self {
            mut conversation_message,
            ds_timestamp,
        } = self;

        conversation_message
            .mark_as_sent(connection, notifier, ds_timestamp)
            .await?;
        Conversation::mark_as_read_until_message_id(
            connection,
            notifier,
            conversation_message.conversation_id(),
            conversation_message.id(),
        )
        .await?;

        Ok(conversation_message)
    }
}
