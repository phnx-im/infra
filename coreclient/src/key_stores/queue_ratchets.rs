// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::utils::persistence::{PersistableStruct, SqlKey};

use super::*;

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub(crate) enum QueueType {
    As,
    Qs,
}

impl std::fmt::Display for QueueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueueType::As => write!(f, "as"),
            QueueType::Qs => write!(f, "qs"),
        }
    }
}

pub(crate) struct QueueRatchetStore<'a> {
    db_connection: &'a Connection,
}

impl<'a> From<&'a Connection> for QueueRatchetStore<'a> {
    fn from(db_connection: &'a Connection) -> Self {
        Self { db_connection }
    }
}

impl<'a> QueueRatchetStore<'a> {
    fn initialize_sequence_number(&self, queue_type: QueueType) -> Result<(), PersistenceError> {
        log::info!("Initializing sequence number for queue type {}", queue_type);
        let sequence_number = PersistableSequenceNumber::from_connection_and_payload(
            self.db_connection,
            QualifiedSequenceNumber {
                queue_type,
                sequence_number: 0,
            },
        );
        sequence_number.persist()
    }

    pub(crate) fn initialize_as_queue_ratchet(&self, ratcht_secret: RatchetSecret) -> Result<()> {
        let payload = AsQueueRatchet::try_from(ratcht_secret)?;
        let ratchet =
            PersistableAsQueueRatchet::from_connection_and_payload(self.db_connection, payload);
        ratchet.persist()?;
        self.initialize_sequence_number(QueueType::As)?;
        Ok(())
    }

    pub(crate) fn initialize_qs_queue_ratchet(&self, ratcht_secret: RatchetSecret) -> Result<()> {
        let ratchet = QsQueueRatchet::try_from(ratcht_secret)?;
        let ratchet =
            PersistableQsQueueRatchet::from_connection_and_payload(self.db_connection, ratchet);
        ratchet.persist()?;
        self.initialize_sequence_number(QueueType::Qs)?;
        Ok(())
    }

    pub(crate) fn get_as_queue_ratchet(&self) -> Result<PersistableAsQueueRatchet> {
        PersistableAsQueueRatchet::load_one(self.db_connection, Some(&QueueType::As), None)?
            .ok_or(anyhow!("Couldn't find AS queue ratchet in DB."))
    }

    pub(crate) fn get_qs_queue_ratchet(&self) -> Result<PersistableQsQueueRatchet> {
        PersistableQsQueueRatchet::load_one(self.db_connection, Some(&QueueType::Qs), None)?
            .ok_or(anyhow!("Couldn't find QS queue ratchet in DB."))
    }

    pub(crate) fn get_sequence_number(
        &self,
        queue_type: QueueType,
    ) -> Result<PersistableSequenceNumber> {
        PersistableSequenceNumber::load_one(self.db_connection, Some(&queue_type), None)?
            .ok_or(anyhow!("No sequence number found for type {}", queue_type))
    }
}

pub(crate) type PersistableAsQueueRatchet<'a> = PersistableStruct<'a, AsQueueRatchet>;

impl PersistableAsQueueRatchet<'_> {
    pub(crate) fn decrypt(&mut self, ciphertext: QueueMessage) -> Result<AsQueueMessagePayload> {
        let plaintext = self.payload.decrypt(ciphertext)?;
        self.persist()?;
        Ok(plaintext)
    }
}

impl SqlKey for QueueType {
    fn to_sql_key(&self) -> String {
        self.to_string()
    }
}

impl Persistable for AsQueueRatchet {
    type Key = QueueType;

    type SecondaryKey = QueueType;

    const DATA_TYPE: DataType = DataType::QueueRatchet;

    fn key(&self) -> &Self::Key {
        &QueueType::As
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &QueueType::As
    }
}

pub(crate) type PersistableQsQueueRatchet<'a> = PersistableStruct<'a, QsQueueRatchet>;

impl PersistableQsQueueRatchet<'_> {
    pub(crate) fn decrypt(&mut self, ciphertext: QueueMessage) -> Result<QsQueueMessagePayload> {
        let plaintext = self.payload.decrypt(ciphertext)?;
        self.persist()?;
        Ok(plaintext)
    }
}

impl Persistable for QsQueueRatchet {
    type Key = QueueType;

    type SecondaryKey = QueueType;

    const DATA_TYPE: DataType = DataType::QueueRatchet;

    fn key(&self) -> &Self::Key {
        &QueueType::Qs
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &QueueType::Qs
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct QualifiedSequenceNumber {
    queue_type: QueueType,
    sequence_number: u64,
}

impl Deref for QualifiedSequenceNumber {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.sequence_number
    }
}

pub(crate) type PersistableSequenceNumber<'a> = PersistableStruct<'a, QualifiedSequenceNumber>;

impl PersistableSequenceNumber<'_> {
    pub(crate) fn set(&mut self, sequence_number: u64) -> Result<()> {
        self.payload.sequence_number = sequence_number;
        self.persist()?;
        Ok(())
    }
}

impl Persistable for QualifiedSequenceNumber {
    type Key = QueueType;

    type SecondaryKey = QueueType;

    const DATA_TYPE: DataType = DataType::SequenceNumber;

    fn key(&self) -> &Self::Key {
        &self.queue_type
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &self.queue_type
    }
}
