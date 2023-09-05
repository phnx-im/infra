// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{
    types::*,
    utils::persistance::{DataType, Persistable},
};

impl Persistable for Conversation {
    type Key = UuidBytes;
    type SecondaryKey = GroupIdBytes;

    const DATA_TYPE: DataType = DataType::Conversation;

    fn own_client_id_bytes(&self) -> Vec<u8> {
        self.own_client_id.clone()
    }

    fn rowid(&self) -> Option<i64> {
        self.rowid
    }

    fn key(&self) -> &Self::Key {
        &self.id
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &self.group_id
    }
}

impl Persistable for ConversationMessage {
    type Key = UuidBytes;

    type SecondaryKey = UuidBytes;

    const DATA_TYPE: DataType = DataType::Message;

    fn own_client_id_bytes(&self) -> Vec<u8> {
        self.own_client_id.clone()
    }

    fn rowid(&self) -> Option<i64> {
        self.rowid
    }

    fn key(&self) -> &Self::Key {
        &self.id
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &self.id
    }
}
