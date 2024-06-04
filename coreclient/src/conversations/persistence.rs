// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use rusqlite::{params, OptionalExtension};

use crate::{utils::persistence::{GroupIdRefWrapper, GroupIdWrapper, Storable}, Conversation, ConversationAttributes};

use super::ConversationPayload;

impl Storable for Conversation {
    const CREATE_TABLE_STATEMENT: &'static str = "
        CREATE TABLE IF NOT EXISTS conversations (
            conversation_id BLOB PRIMARY KEY,
            conversation_title TEXT NOT NULL,
            conversation_picture BLOB,
            group_id BLOB UNIQUE NOT NULL,
            last_used TEXT NOT NULL,
            last_read TEXT NOT NULL,
            conversation_status TEXT NOT NULL CHECK (conversation_status LIKE 'active' OR conversation_status LIKE 'inactive:*'),
            conversation_type TEXT NOT NULL CHECK (conversation_type LIKE 'group' OR conversation_type LIKE 'unconfirmed_connection:*' OR conversation_type LIKE 'connection:*'),
        );";
        
            fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error>
         {
        let id = row.get(0)?;
        let conversation_title = row.get(1)?;
        let conversation_picture_option = row.get(2)?;
        let group_id: GroupIdWrapper = row.get(3)?;
        let last_used = row.get(4)?;
        let last_read = row.get(5)?;
        let status = row.get(6)?;
        let conversation_type = row.get(7)?;

        let conversation_payload = ConversationPayload {
            status,
            conversation_type,
            attributes: ConversationAttributes {
                title: conversation_title,
                conversation_picture_option,
            },
        };

        Ok(Conversation {
            id,
            group_id: group_id.into(),
            last_used,
            last_read,
            conversation_payload,
        })

            }
}

impl Conversation {
    pub(crate) fn store(&self, connection: &rusqlite::Connection) -> rusqlite::Result<()> {
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

    pub(crate) fn update(&self, connection: &rusqlite::Connection) -> rusqlite::Result<()> {
        let group_id = GroupIdRefWrapper::from(&self.group_id);
        connection.execute(
            "UPDATE conversations SET conversation_title = ?, conversation_picture = ?, group_id = ?, last_used = ?, last_read = ?, conversation_status = ?, conversation_type = ? WHERE conversation_id = ?",
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

    pub(crate) fn load(connection: &rusqlite::Connection, conversation_id: &str) -> Result<Option<Conversation>, rusqlite::Error> {
        let mut stmt = connection.prepare("SELECT conversation_id, conversation_title, conversation_picture, group_id, last_used, last_read, conversation_status, conversation_type FROM conversations WHERE conversation_id = ?")?;
        stmt.query_row(params![conversation_id], Self::from_row).optional()

    }
}
