// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use aircommon::{
    credentials::keys::ClientSigningKey,
    identifiers::{MimiId, UserId},
    messages::client_ds_out::SendMessageParamsOut,
    time::TimeStamp,
};
use anyhow::{Context, bail};
use mimi_content::{
    ByteBuf, Disposition, MessageStatus, MessageStatusReport, MimiContent, NestedPart,
    NestedPartContent, PerMessageStatus,
};
use openmls::storage::OpenMlsProvider;
use sqlx::{SqliteConnection, SqliteTransaction};
use uuid::Uuid;

use crate::{
    ContentMessage, Conversation, ConversationId, ConversationMessage, ConversationMessageId,
    Message,
    conversations::{StatusRecord, messages::edit::MessageEdit},
};

use super::{AirOpenMlsProvider, ApiClients, CoreUser, Group, StoreNotifier};

impl CoreUser {
    /// Send a message and return it.
    ///
    /// The message unsent messages is stored, then sent to the DS and finally returned. The
    /// conversation is marked as read until this message.
    pub(crate) async fn send_message(
        &self,
        conversation_id: ConversationId,
        content: MimiContent,
        replaces_id: Option<ConversationMessageId>,
    ) -> anyhow::Result<ConversationMessage> {
        let needs_update = self
            .with_transaction(async |txn| {
                let conversation = Conversation::load(txn.as_mut(), &conversation_id)
                    .await?
                    .with_context(|| {
                        format!("Can't find conversation with id {conversation_id}")
                    })?;
                let group_id = conversation.group_id;
                let group = Group::load_clean(txn, &group_id)
                    .await?
                    .with_context(|| format!("Can't find group with id {group_id:?}"))?;
                Ok(group.mls_group().has_pending_proposals())
            })
            .await?;

        if needs_update {
            // TODO race condition: Before or after this update, new proposals could arrive
            self.update_key(conversation_id).await?;
        }

        let unsent_group_message = self
            .with_transaction_and_notifier(async |txn, notifier| {
                UnsentContent {
                    conversation_id,
                    conversation_message_id: ConversationMessageId::random(),
                    content,
                }
                .store_unsent_message(txn, notifier, self.user_id(), replaces_id)
                .await?
                .create_group_message(&AirOpenMlsProvider::new(txn), self.signing_key())?
                .store_group_update(txn, notifier, self.user_id())
                .await
            })
            .await?;

        let sent_message = unsent_group_message
            .send_message_to_ds(&self.inner.api_clients, self.signing_key())
            .await?;

        self.with_transaction_and_notifier(async |txn, notifier| {
            sent_message
                .mark_as_sent(txn, notifier, self.user_id())
                .await
        })
        .await
    }

    pub(crate) async fn send_message_transactional(
        &self,
        txn: &mut SqliteTransaction<'_>,
        notifier: &mut StoreNotifier,
        conversation_id: ConversationId,
        conversation_message_id: ConversationMessageId,
        content: MimiContent,
    ) -> anyhow::Result<ConversationMessage> {
        let unsent_group_message = UnsentContent {
            conversation_id,
            conversation_message_id,
            content,
        }
        .store_unsent_message(txn, notifier, self.user_id(), None)
        .await?
        .create_group_message(&AirOpenMlsProvider::new(txn), self.signing_key())?
        .store_group_update(txn, notifier, self.user_id())
        .await?;

        let sent_message = unsent_group_message
            .send_message_to_ds(&self.inner.api_clients, self.signing_key())
            .await?;

        sent_message
            .mark_as_sent(txn, notifier, self.user_id())
            .await
    }

    /// Re-try sending a message, where sending previously failed.
    pub async fn re_send_message(&self, local_message_id: Uuid) -> anyhow::Result<()> {
        let unsent_group_message = self
            .with_transaction(async |txn| {
                LocalMessage { local_message_id }
                    .load_for_resend(txn)
                    .await?
                    .create_group_message(&AirOpenMlsProvider::new(txn), self.signing_key())
            })
            .await?;

        let sent_message = unsent_group_message
            .send_message_to_ds(&self.inner.api_clients, self.signing_key())
            .await?;

        self.with_transaction_and_notifier(async |connection, notifier| {
            // Do not mark as read, because the user might have missed messages
            sent_message
                .mark_as_sent(connection, notifier, self.user_id())
                .await
        })
        .await?;

        Ok(())
    }

    pub(crate) async fn send_delivery_receipts(
        &self,
        conversation_id: ConversationId,
        statuses: impl IntoIterator<Item = (&MimiId, MessageStatus)>,
    ) -> anyhow::Result<()> {
        let Some(unsent_receipt) = UnsentReceipt::new(statuses)? else {
            return Ok(()); // Nothing to send
        };

        let (conversation, group, params) = self
            .with_transaction(async |txn| {
                let conversation = Conversation::load(&mut *txn, &conversation_id)
                    .await?
                    .with_context(|| {
                        format!("Can't find conversation with id {conversation_id}")
                    })?;
                let group_id = conversation.group_id();
                let mut group = Group::load_clean(txn, group_id)
                    .await?
                    .with_context(|| format!("Can't find group with id {group_id:?}"))?;
                let params = group.create_message(
                    &AirOpenMlsProvider::new(txn),
                    self.signing_key(),
                    unsent_receipt.content,
                )?;
                group.store_update(txn.as_mut()).await?;
                Ok((conversation, group, params))
            })
            .await?;

        self.inner
            .api_clients
            .get(&conversation.owner_domain())?
            .ds_send_message(params, self.signing_key(), group.group_state_ear_key())
            .await?;

        self.with_transaction_and_notifier(async |txn, notifier| {
            StatusRecord::borrowed(self.user_id(), unsent_receipt.report, TimeStamp::now())
                .store_report(txn, notifier)
                .await?;
            Ok(())
        })
        .await?;

        Ok(())
    }
}

struct UnsentContent {
    conversation_id: ConversationId,
    conversation_message_id: ConversationMessageId,
    content: MimiContent,
}

impl UnsentContent {
    async fn store_unsent_message(
        self,
        txn: &mut SqliteTransaction<'_>,
        notifier: &mut StoreNotifier,
        sender: &UserId,
        replaces_id: Option<ConversationMessageId>,
    ) -> anyhow::Result<UnsentMessage<WithContent, GroupUpdateNeeded>> {
        let UnsentContent {
            conversation_id,
            conversation_message_id,
            mut content,
        } = self;

        let conversation = Conversation::load(txn.as_mut(), &conversation_id)
            .await?
            .with_context(|| format!("Can't find conversation with id {conversation_id}"))?;

        let is_deletion = content.nested_part.part == NestedPartContent::NullPart;

        let conversation_message = if let Some(replaces_id) = replaces_id {
            // Load the original message and the Mimi ID of the original message
            let mut original = ConversationMessage::load(txn.as_mut(), replaces_id)
                .await?
                .with_context(|| format!("Can't find message with id {replaces_id:?}"))?;
            let original_mimi_content = original
                .message()
                .mimi_content()
                .context("Replaced message does not have mimi content")?;
            let original_mimi_id = original
                .message()
                .mimi_id()
                .context("Replaced message does not have mimi id")?;
            content.replaces = Some(original_mimi_id.as_slice().to_vec().into());
            let edit_created_at = TimeStamp::now();

            if !is_deletion {
                // Store the edit
                let edit = MessageEdit::new(
                    original_mimi_id,
                    original.id(),
                    edit_created_at,
                    original_mimi_content,
                );
                edit.store(txn.as_mut()).await?;
            }

            // Edit the original message and clear its status
            let is_sent = false;
            original.set_content_message(ContentMessage::new(
                sender.clone(),
                is_sent,
                content.clone(),
                conversation.group_id(),
            ));
            if is_deletion {
                original.set_status(MessageStatus::Deleted);
            } else {
                original.set_status(MessageStatus::Unread);
            }
            original.set_edited_at(edit_created_at);
            original.update(txn.as_mut(), notifier).await?;
            StatusRecord::clear(txn.as_mut(), notifier, original.id()).await?;

            original
        } else {
            // Store the message as unsent so that we don't lose it in case
            // something goes wrong.
            let conversation_message = ConversationMessage::new_unsent_message(
                sender.clone(),
                conversation_id,
                conversation_message_id,
                content.clone(),
                conversation.group_id(),
            );
            conversation_message.store(txn.as_mut(), notifier).await?;
            conversation_message
        };

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
        signer: &ClientSigningKey,
    ) -> anyhow::Result<UnsentMessage<WithParams, GroupUpdate>> {
        let Self {
            conversation,
            mut group,
            conversation_message,
            content: WithContent(content),
            group_update,
        } = self;

        let params = group.create_message(provider, signer, content)?;

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
        txn: &mut SqliteTransaction<'_>,
        notifier: &mut StoreNotifier,
        own_user: &UserId,
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
        group.store_update(txn.as_mut()).await?;

        // Also, mark the message (and all messages preceeding it) as read.
        Conversation::mark_as_read_until_message_id(
            txn,
            notifier,
            conversation.id(),
            conversation_message.id(),
            own_user,
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
    async fn send_message_to_ds(
        self,
        api_clients: &ApiClients,
        signer: &ClientSigningKey,
    ) -> anyhow::Result<SentMessage> {
        let Self {
            conversation,
            conversation_message,
            group,
            content: WithParams(params),
            group_update: GroupUpdated,
        } = self;

        let ds_timestamp = api_clients
            .get(&conversation.owner_domain())?
            .ds_send_message(params, signer, group.group_state_ear_key())
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
    async fn mark_as_sent(
        self,
        txn: &mut SqliteTransaction<'_>,
        notifier: &mut StoreNotifier,
        own_user: &UserId,
    ) -> anyhow::Result<ConversationMessage> {
        let Self {
            mut conversation_message,
            ds_timestamp,
        } = self;

        if conversation_message.edited_at().is_some() {
            conversation_message
                .mark_as_sent(&mut *txn, notifier, conversation_message.timestamp().into())
                .await?;
            conversation_message.set_edited_at(ds_timestamp);
        } else {
            conversation_message
                .mark_as_sent(&mut *txn, notifier, ds_timestamp)
                .await?;
        }

        // Note: even though the message was already marked as read, we still need to move the last
        // read timestamp down. After a message was sent to DS, it is marked as read, which updates
        // its timestamp to the timestamp returned by DS.
        Conversation::mark_as_read_until_message_id(
            txn,
            notifier,
            conversation_message.conversation_id(),
            conversation_message.id(),
            own_user,
        )
        .await?;

        Ok(conversation_message)
    }
}

/// Not yet sent receipt message consisting of the content to send and a local message status
/// report.
struct UnsentReceipt {
    report: MessageStatusReport,
    content: MimiContent,
}

impl UnsentReceipt {
    fn new<'a>(
        statuses: impl IntoIterator<Item = (&'a MimiId, MessageStatus)>,
    ) -> anyhow::Result<Option<Self>> {
        let report = MessageStatusReport {
            statuses: statuses
                .into_iter()
                .map(|(id, status)| PerMessageStatus {
                    mimi_id: id.as_ref().to_vec().into(),
                    status,
                })
                .collect(),
        };

        if report.statuses.is_empty() {
            return Ok(None);
        }

        let content = MimiContent {
            salt: ByteBuf::from(aircommon::crypto::secrets::Secret::<16>::random()?.secret()),
            nested_part: NestedPart {
                disposition: Disposition::Unspecified,
                part: NestedPartContent::SinglePart {
                    content_type: "application/mimi-message-status".to_owned(),
                    content: report.serialize()?.into(),
                },
                ..Default::default()
            },
            ..Default::default()
        };

        Ok(Some(Self { report, content }))
    }
}
