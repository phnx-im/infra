// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::utils::persistance::{DataType, Persistable};

use super::*;

impl Persistable for Group {
    type Key = GroupId;
    type SecondaryKey = GroupId;

    const DATA_TYPE: DataType = DataType::Group;

    fn own_client_id_bytes(&self) -> Vec<u8> {
        self.own_client_id.tls_serialize_detached().unwrap()
    }

    fn rowid(&self) -> Option<i64> {
        self.rowid
    }

    fn key(&self) -> &Self::Key {
        self.group_id()
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        self.group_id()
    }

    fn set_rowid(&mut self, rowid: i64) {
        self.rowid = Some(rowid);
    }
}
