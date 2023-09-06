// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use tls_codec::Serialize;

use crate::utils::persistance::{DataType, Persistable};

use super::*;

impl Persistable for Contact {
    type Key = UserName;
    type SecondaryKey = UserName;
    const DATA_TYPE: DataType = DataType::Contact;

    fn own_client_id_bytes(&self) -> Vec<u8> {
        self.own_client_id.tls_serialize_detached().unwrap()
    }

    fn rowid(&self) -> Option<i64> {
        self.rowid
    }

    fn key(&self) -> &Self::Key {
        &self.user_name
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &self.user_name
    }

    fn set_rowid(&mut self, rowid: i64) {
        self.rowid = Some(rowid);
    }
}
