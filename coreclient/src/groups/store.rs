// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use turbosql::{execute, select, Turbosql};

use super::*;

#[derive(Turbosql)]
struct TurboGroup {
    rowid: Option<i64>,
    // We store the group and client id as a byte vector to be able to use it as
    // a SQL key.
    client_id: Option<Vec<u8>>,
    group_id: Option<Vec<u8>>,
    group_bytes: Option<Vec<u8>>,
}

impl TryFrom<TurboGroup> for Group {
    type Error = turbosql::Error;

    fn try_from(value: TurboGroup) -> Result<Self, Self::Error> {
        if let Some(group_bytes) = value.group_bytes {
            let mut group: Group = serde_json::from_slice(&group_bytes)?;
            group.rowid = value.rowid;
            Ok(group)
        } else {
            Err(turbosql::Error::OtherError("Corrupted group in DB."))
        }
    }
}

impl TryFrom<&Group> for TurboGroup {
    type Error = turbosql::Error;

    fn try_from(value: &Group) -> Result<Self, Self::Error> {
        let group = Self {
            rowid: value.rowid,
            client_id: Some(
                value
                    .own_client_id()
                    .tls_serialize_detached()
                    .map_err(|_| turbosql::Error::OtherError("Could not serialize client id."))?,
            ),
            group_id: Some(value.group_id.as_slice().to_vec()),
            group_bytes: Some(serde_json::to_vec(value)?),
        };
        Ok(group)
    }
}

impl Group {
    pub(crate) fn load(
        group_id: &GroupId,
        client_id: &AsClientId,
    ) -> Result<Self, turbosql::Error> {
        let client_id_bytes = client_id.tls_serialize_detached().unwrap();
        let group_id_bytes = group_id.as_slice();
        let turbo_group = select!(TurboGroup "WHERE client_id = " client_id_bytes " AND group_id = " group_id_bytes)?;
        turbo_group.try_into()
    }

    pub(crate) fn persist(&self) -> Result<(), turbosql::Error> {
        if self.rowid.is_some() {
            let turbo_group: TurboGroup = self.try_into()?;
            turbo_group.update()?;
        } else {
            let turbo_group: TurboGroup = self.try_into()?;
            // We can unwrap these, as they are both set in the constructor.
            let client_id_bytes = turbo_group.client_id.as_ref().unwrap();
            let group_id_bytes = turbo_group.group_id.as_ref().unwrap();
            // Check if a group with this ID already exists.
            if let Ok(old_group) = select!(TurboGroup "WHERE client_id = " client_id_bytes " AND group_id = " group_id_bytes)
            {
                // If it exists, delete it from the DB. (We could probably just
                // read out the rowid of the existing group and set it for the
                // new group, but this does the trick.)
                execute!("DELETE FROM turbogroup WHERE rowid = " old_group.rowid.unwrap())?;
            }
            // Insert the new group into the DB.
            turbo_group.insert()?;
        }
        Ok(())
    }

    pub(crate) fn _purge(&self) -> Result<(), turbosql::Error> {
        let turbo_group: TurboGroup = self.try_into()?;
        let rowid = turbo_group.rowid.ok_or(turbosql::Error::OtherError(
            "Cannot purge group without rowid.",
        ))?;
        execute!("DELETE FROM turbogroup WHERE rowid = " rowid)?;
        Ok(())
    }
}
