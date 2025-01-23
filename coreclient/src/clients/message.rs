// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Context;
use phnxtypes::{
    identifiers::QualifiedUserName, messages::client_ds_out::SendMessageParamsOut, time::TimeStamp,
};
use rusqlite::{Connection, Transaction};

use crate::{Conversation, ConversationId, ConversationMessage, MimiContent};

use super::{ApiClients, CoreUser, Group, StoreNotifier};

impl CoreUser {
    /// Send a message and return it.
    ///
    /// Note that the message has already been sent to the DS and has internally been stored in the
    /// conversation store.
    pub(crate) async fn send_message(
        &self,
        conversation_id: ConversationId,
        content: MimiContent,
    ) -> anyhow::Result<ConversationMessage> {
        let unsent_group_message = self
            .with_transaction(|transaction| {
                InitialParams {
                    conversation_id,
                    content,
                }
                .store_unsent_message(transaction, self.store_notifier(), &self.user_name())?
                .create_unsent_group_message(transaction, self.store_notifier())
            })
            .await?;

        let sent_message = unsent_group_message
            .send_message_to_ds(&self.inner.api_clients)
            .await?;

        self.with_transaction(|transaction| {
            sent_message.mark_as_sent_and_read(transaction, self.store_notifier())
        })
        .await
    }
}

struct InitialParams {
    conversation_id: ConversationId,
    content: MimiContent,
}

impl InitialParams {
    fn store_unsent_message(
        self,
        connection: &Connection,
        mut notifier: StoreNotifier,
        sender: &QualifiedUserName,
    ) -> anyhow::Result<UnsentMessage> {
        let InitialParams {
            conversation_id,
            content,
        } = self;

        let conversation = Conversation::load(connection, &conversation_id)?
            .with_context(|| format!("Can't find conversation with id {conversation_id}"))?;
        // Store the message as unsent so that we don't lose it in case
        // something goes wrong.
        let conversation_message = ConversationMessage::new_unsent_message(
            sender.to_string(),
            conversation_id,
            content.clone(),
        );
        conversation_message.store(connection, &mut notifier)?;

        // Notify as early as possible to react to the not yet sent message
        notifier.notify();

        Ok(UnsentMessage {
            conversation,
            conversation_message,
            content,
        })
    }
}

struct UnsentMessage {
    conversation: Conversation,
    conversation_message: ConversationMessage,
    content: MimiContent,
}

impl UnsentMessage {
    fn create_unsent_group_message(
        self,
        transaction: &Transaction,
        mut notifier: StoreNotifier,
    ) -> anyhow::Result<UnsentGroupMessage> {
        let Self {
            conversation,
            conversation_message,
            content,
        } = self;

        let group_id = conversation.group_id();
        let mut group = Group::load(transaction, group_id)?
            .with_context(|| format!("Can't find group with id {group_id:?}"))?;

        let params = group.create_message(transaction, content)?;
        // Immediately write the group back. No need to wait for the DS to
        // confirm as this is just an application message.
        group.store_update(transaction)?;
        // Also, mark the message (and all messages preceeding it) as read.
        Conversation::mark_as_read_until_message_id(
            transaction,
            &mut notifier,
            conversation.id(),
            conversation_message.id(),
        )?;
        notifier.notify();

        Ok(UnsentGroupMessage {
            conversation,
            conversation_message,
            group,
            params,
        })
    }
}

struct UnsentGroupMessage {
    conversation: Conversation,
    conversation_message: ConversationMessage,
    group: Group,
    params: SendMessageParamsOut,
}

impl UnsentGroupMessage {
    async fn send_message_to_ds(self, api_clients: &ApiClients) -> anyhow::Result<SentMessage> {
        let Self {
            conversation,
            conversation_message,
            group,
            params,
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
    fn mark_as_sent_and_read(
        self,
        transaction: &Transaction,
        mut notifier: StoreNotifier,
    ) -> anyhow::Result<ConversationMessage> {
        let Self {
            mut conversation_message,
            ds_timestamp,
        } = self;

        conversation_message.mark_as_sent(transaction, &mut notifier, ds_timestamp)?;
        Conversation::mark_as_read_until_message_id(
            transaction,
            &mut notifier,
            conversation_message.conversation_id(),
            conversation_message.id(),
        )?;

        notifier.notify();

        Ok(conversation_message)
    }
}
