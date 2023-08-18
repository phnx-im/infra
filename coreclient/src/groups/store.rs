// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use turbosql::{execute, select, Turbosql};

use super::*;

impl ClientGroup {
    /// Writes the new group to the database. If a group with the same group id
    /// already exists, it deletes the old group.
    pub(crate) fn new(inner_group: InnerClientGroup) -> Result<Self, turbosql::Error> {
        let client_id_bytes = inner_group
            .client_by_index(inner_group.mls_group().own_leaf_index().usize())
            .unwrap()
            .tls_serialize_detached()
            .unwrap();
        let group_id_bytes = inner_group.group_id().as_slice().to_vec();
        // Check if a group with this ID already exists.
        if let Ok(old_group) = select!(ClientGroup "WHERE client_id = " client_id_bytes " AND group_id = " group_id_bytes)
        {
            // If it exists, delete it from the DB.
            execute!("DELETE FROM clientgroup WHERE rowid = " old_group.rowid.unwrap())?;
        }
        // Insert the new group into the DB.
        let group = Self {
            rowid: None,
            client_id: Some(client_id_bytes),
            group_id: Some(group_id_bytes),
            inner_client_group: Some(inner_group),
        };
        group.insert()?;
        Ok(group)
    }

    pub(crate) fn load(
        group_id: &GroupId,
        client_id: &AsClientId,
    ) -> Result<Self, turbosql::Error> {
        let client_id_bytes = client_id.tls_serialize_detached().unwrap();
        let group_id_bytes = group_id.as_slice();
        let group = select!(ClientGroup "WHERE client_id = " client_id_bytes " AND group_id = " group_id_bytes)?;
        if group.inner_client_group.is_none() {
            return Err(turbosql::Error::OtherError("Corrupted group in DB."));
        }
        Ok(group)
    }

    pub(crate) fn persist(&self) -> Result<(), turbosql::Error> {
        <Self as Turbosql>::update(self)?;
        Ok(())
    }

    pub(crate) fn _purge(&self) -> Result<(), turbosql::Error> {
        execute!("DELETE FROM clientgroup WHERE rowid = " self.rowid.unwrap())?;
        Ok(())
    }
}
