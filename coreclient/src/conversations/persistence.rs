// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls::group::GroupId;
use phnxtypes::time::TimeStamp;
use rusqlite::{named_params, params, Connection, OptionalExtension, Transaction};

use crate::{
    utils::persistence::{GroupIdRefWrapper, GroupIdWrapper, Storable},
    Conversation, ConversationAttributes, ConversationId, ConversationStatus, ConversationType,
};

impl Storable for Conversation {
    const CREATE_TABLE_STATEMENT: &'static str = "
        CREATE TABLE IF NOT EXISTS conversations (
            conversation_id BLOB PRIMARY KEY,
            conversation_title TEXT NOT NULL,
            conversation_picture BLOB,
            group_id BLOB NOT NULL,
            last_used TEXT NOT NULL,
            last_read TEXT NOT NULL,
            conversation_status TEXT NOT NULL CHECK (conversation_status LIKE 'active' OR conversation_status LIKE 'inactive:%'),
            conversation_type TEXT NOT NULL CHECK (conversation_type LIKE 'group' OR conversation_type LIKE 'unconfirmed_connection:%' OR conversation_type LIKE 'connection:%')
        );";

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        let id = row.get(0)?;
        let conversation_title = row.get(1)?;
        let conversation_picture_option = row.get(2)?;
        let group_id: GroupIdWrapper = row.get(3)?;
        let last_used = row.get(4)?;
        let last_read = row.get(5)?;
        let status = row.get(6)?;
        let conversation_type = row.get(7)?;

        Ok(Conversation {
            id,
            group_id: group_id.into(),
            last_used,
            last_read,
            status,
            conversation_type,
            attributes: ConversationAttributes {
                title: conversation_title,
                conversation_picture_option,
            },
        })
    }
}

impl Conversation {
    pub(crate) fn store(&self, connection: &Connection) -> rusqlite::Result<()> {
        log::info!("Storing conversation: {:?}", self.id);
        log::info!("With title: {:?}", self.attributes().title());
        let group_id = GroupIdRefWrapper::from(&self.group_id);
        connection.execute(
            "INSERT INTO conversations (conversation_id, conversation_title, conversation_picture, group_id, last_used, last_read, conversation_status, conversation_type) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                self.id,
                self.attributes().title(),
                self.attributes().conversation_picture_option(),
                group_id,
                self.last_used,
                self.last_read,
                self.status(),
                self.conversation_type(),
            ],
        )?;
        Ok(())
    }

    pub(crate) fn load(
        connection: &Connection,
        conversation_id: &ConversationId,
    ) -> Result<Option<Conversation>, rusqlite::Error> {
        let mut stmt = connection.prepare("SELECT conversation_id, conversation_title, conversation_picture, group_id, last_used, last_read, conversation_status, conversation_type FROM conversations WHERE conversation_id = ?")?;
        stmt.query_row(params![conversation_id], Self::from_row)
            .optional()
    }

    pub(crate) fn load_by_group_id(
        connection: &Connection,
        group_id: &GroupId,
    ) -> Result<Option<Conversation>, rusqlite::Error> {
        let group_id = GroupIdRefWrapper::from(group_id);
        let mut stmt = connection.prepare("SELECT conversation_id, conversation_title, conversation_picture, group_id, last_used, last_read, conversation_status, conversation_type FROM conversations WHERE group_id = ?")?;
        stmt.query_row(params![group_id], Self::from_row).optional()
    }

    pub(crate) fn load_all(connection: &Connection) -> Result<Vec<Conversation>, rusqlite::Error> {
        let mut stmt = connection.prepare("SELECT conversation_id, conversation_title, conversation_picture, group_id, last_used, last_read, conversation_status, conversation_type FROM conversations")?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect()
    }

    pub(super) fn update_conversation_picture(
        &self,
        connection: &Connection,
        conversation_picture: Option<&[u8]>,
    ) -> rusqlite::Result<()> {
        connection.execute(
            "UPDATE conversations SET conversation_picture = ? WHERE conversation_id = ?",
            params![conversation_picture, self.id],
        )?;
        Ok(())
    }

    pub(super) fn update_status(
        &self,
        connection: &Connection,
        status: &ConversationStatus,
    ) -> rusqlite::Result<()> {
        connection.execute(
            "UPDATE conversations SET conversation_status = ? WHERE conversation_id = ?",
            params![status, self.id],
        )?;
        Ok(())
    }

    pub(crate) fn delete(
        connection: &Connection,
        conversation_id: ConversationId,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "DELETE FROM conversations WHERE conversation_id = ?",
            params![conversation_id],
        )?;
        Ok(())
    }

    /// Set the `last_read` marker of all conversations with the given
    /// [`ConversationId`]s to the given timestamps. This is used to mark all
    /// messages up to this timestamp as read.
    pub(crate) fn mark_as_read<
        'b,
        T: 'b + IntoIterator<Item = (&'b ConversationId, &'b TimeStamp)>,
    >(
        transaction: &mut Transaction,
        mark_as_read_data: T,
    ) -> Result<(), rusqlite::Error> {
        for (conversation_id, timestamp) in mark_as_read_data.into_iter() {
            transaction.execute(
                "UPDATE conversations SET last_read = ? WHERE conversation_id = ?",
                params![timestamp, conversation_id],
            )?;
        }
        Ok(())
    }

    pub(crate) fn unread_message_count(
        connection: &Connection,
        conversation_id: ConversationId,
    ) -> Result<u32, rusqlite::Error> {
        connection.query_row(
            "SELECT COUNT(*) FROM conversation_messages WHERE conversation_id = :conversation_id AND timestamp > (SELECT last_read FROM conversations WHERE conversation_id = :conversation_id)",
            named_params! {":conversation_id": conversation_id},
            |row| row.get(0),
        )
    }

    pub(super) fn set_conversation_type(
        &self,
        connection: &Connection,
        conversation_type: &ConversationType,
    ) -> rusqlite::Result<()> {
        connection.execute(
            "UPDATE conversations SET conversation_type = ? WHERE conversation_id = ?",
            params![conversation_type, self.id],
        )?;
        Ok(())
    }
}
