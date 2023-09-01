// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::utils::persistance::{DataType, Persistable};

use super::*;

impl Persistable for Contact {
    type Key = UserName;
    const DATA_TYPE: DataType = DataType::Contact;

    fn own_client_id(&self) -> &AsClientId {
        &self.own_client_id
    }

    fn rowid(&self) -> Option<i64> {
        self.rowid
    }

    fn key(&self) -> &Self::Key {
        &self.user_name
    }
}
