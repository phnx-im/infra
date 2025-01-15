// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ops::DerefMut;

use phnxtypes::{
    crypto::{
        kdf::keys::RatchetSecret,
        ratchet::{QueueRatchet, RatchetCiphertext, RatchetPayload},
    },
    messages::{
        client_as::AsQueueMessagePayload, client_ds::QsQueueMessagePayload,
        EncryptedAsQueueMessage, EncryptedQsQueueMessage,
    },
};
use rusqlite::params;
use tracing::error;

use crate::utils::persistence::Storable;

use super::*;

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub(crate) enum QueueType {
    As,
    Qs,
}

impl QueueType {
    pub(crate) fn load_sequence_number(
        &self,
        connection: &Connection,
    ) -> Result<u64, rusqlite::Error> {
        let mut stmt = connection
            .prepare("SELECT sequence_number FROM queue_ratchets WHERE queue_type = ?;")?;
        stmt.query_row(params![self.to_string()], |row| row.get(0))
    }

    pub(crate) fn update_sequence_number(
        &self,
        connection: &Connection,
        sequence_number: u64,
    ) -> Result<(), rusqlite::Error> {
        let mut stmt = connection
            .prepare("UPDATE queue_ratchets SET sequence_number = ? WHERE queue_type = ?;")?;
        stmt.execute(params![sequence_number, self.to_string()])?;
        Ok(())
    }
}

pub(crate) struct StorableQueueRatchet<
    Ciphertext: RatchetCiphertext,
    Payload: RatchetPayload<Ciphertext>,
> {
    queue_type: QueueType,
    queue_ratchet: QueueRatchet<Ciphertext, Payload>,
}

impl<Ciphertext: RatchetCiphertext, Payload: RatchetPayload<Ciphertext>> Deref
    for StorableQueueRatchet<Ciphertext, Payload>
{
    type Target = QueueRatchet<Ciphertext, Payload>;

    fn deref(&self) -> &Self::Target {
        &self.queue_ratchet
    }
}

impl<Ciphertext: RatchetCiphertext, Payload: RatchetPayload<Ciphertext>> DerefMut
    for StorableQueueRatchet<Ciphertext, Payload>
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.queue_ratchet
    }
}

pub(crate) type StorableQsQueueRatchet =
    StorableQueueRatchet<EncryptedQsQueueMessage, QsQueueMessagePayload>;

impl StorableQsQueueRatchet {
    pub(crate) fn initialize(
        connection: &Connection,
        ratcht_secret: RatchetSecret,
    ) -> Result<(), rusqlite::Error> {
        Self {
            queue_type: QueueType::Qs,
            queue_ratchet: QueueRatchet::try_from(ratcht_secret).map_err(|error| {
                error!(%error, "Error initializing QS queue ratchet");
                // This is just a library error, so we hide it behind a rusqlite
                // error.
                rusqlite::Error::InvalidQuery
            })?,
        }
        .store(connection)?;
        Ok(())
    }

    pub(crate) fn load(connection: &Connection) -> Result<Self, rusqlite::Error> {
        StorableQueueRatchet::load_internal(connection, QueueType::Qs)
    }

    pub(crate) fn update_ratchet(&self, connection: &Connection) -> Result<(), rusqlite::Error> {
        self.update_internal(connection, QueueType::Qs)
    }
}

pub(crate) type StorableAsQueueRatchet =
    StorableQueueRatchet<EncryptedAsQueueMessage, AsQueueMessagePayload>;

impl StorableAsQueueRatchet {
    pub(crate) fn initialize(
        connection: &Connection,
        ratcht_secret: RatchetSecret,
    ) -> Result<(), rusqlite::Error> {
        Self {
            queue_type: QueueType::As,
            queue_ratchet: QueueRatchet::try_from(ratcht_secret).map_err(|error| {
                error!(%error, "Error initializing AS queue ratchet");
                // This is just a library error, so we hide it behind a rusqlite
                // error.
                rusqlite::Error::InvalidQuery
            })?,
        }
        .store(connection)?;
        Ok(())
    }

    pub(crate) fn load(connection: &Connection) -> Result<Self, rusqlite::Error> {
        StorableQueueRatchet::load_internal(connection, QueueType::As)
    }

    pub(crate) fn update_ratchet(&self, connection: &Connection) -> Result<(), rusqlite::Error> {
        self.update_internal(connection, QueueType::As)
    }
}

impl<Ciphertext: RatchetCiphertext, Payload: RatchetPayload<Ciphertext>> Storable
    for StorableQueueRatchet<Ciphertext, Payload>
{
    const CREATE_TABLE_STATEMENT: &'static str = "
        CREATE TABLE IF NOT EXISTS queue_ratchets (
            queue_type TEXT PRIMARY KEY CHECK (queue_type IN ('as', 'qs')),
            queue_ratchet BLOB NOT NULL,
            sequence_number INTEGER NOT NULL DEFAULT 0
        );";

    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        let queue_type_str: String = row.get(0)?;
        let queue_type = match queue_type_str.as_str() {
            "as" => QueueType::As,
            "qs" => QueueType::Qs,
            _ => return Err(rusqlite::Error::InvalidQuery),
        };
        let queue_ratchet = row.get(1)?;
        Ok(Self {
            queue_type,
            queue_ratchet,
        })
    }
}

impl std::fmt::Display for QueueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueueType::As => write!(f, "as"),
            QueueType::Qs => write!(f, "qs"),
        }
    }
}

impl<Ciphertext: RatchetCiphertext, Payload: RatchetPayload<Ciphertext>>
    StorableQueueRatchet<Ciphertext, Payload>
{
    fn store(&self, connection: &Connection) -> Result<(), rusqlite::Error> {
        let mut stmt = connection
            .prepare("INSERT INTO queue_ratchets (queue_type, queue_ratchet) VALUES (?, ?);")?;
        stmt.execute(params![self.queue_type.to_string(), self.queue_ratchet])?;
        Ok(())
    }

    fn load_internal(
        connection: &Connection,
        queue_type: QueueType,
    ) -> Result<Self, rusqlite::Error> {
        let mut stmt = connection.prepare(
            "SELECT queue_type, queue_ratchet FROM queue_ratchets WHERE queue_type = ?;",
        )?;
        stmt.query_row(params![queue_type.to_string()], Self::from_row)
    }

    pub(crate) fn update_internal(
        &self,
        connection: &Connection,
        queue_type: QueueType,
    ) -> Result<(), rusqlite::Error> {
        let mut stmt = connection
            .prepare("UPDATE queue_ratchets SET queue_ratchet = ? WHERE queue_type = ?;")?;
        stmt.execute(params![self.queue_ratchet, queue_type.to_string()])?;
        Ok(())
    }
}
