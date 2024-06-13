// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls_traits::storage::{
    traits::{GroupId as GroupIdTrait, LeafNode as LeafNodeTrait},
    CURRENT_VERSION,
};
use rusqlite::params;

use crate::utils::persistence::Storable;

use super::storage_provider::KeyRefWrapper;

pub(crate) struct OwnLeafNode {
    leaf_node_bytes: Vec<u8>,
}

impl Storable for OwnLeafNode {
    const CREATE_TABLE_STATEMENT: &'static str = "CREATE TABLE IF NOT EXISTS own_leaf_nodes (
        group_id BLOB PRIMARY KEY,
        leaf_node BLOB NOT NULL,
    )";

    fn from_row(row: &rusqlite::Row) -> anyhow::Result<Self, rusqlite::Error> {
        let leaf_node_bytes: Vec<u8> = row.get(0)?;
        Ok(Self { leaf_node_bytes })
    }
}

impl OwnLeafNode {
    pub(super) fn new<LeafNode: LeafNodeTrait<CURRENT_VERSION>>(
        leaf_node: &LeafNode,
    ) -> Result<Self, serde_json::Error> {
        let leaf_node_bytes = serde_json::to_vec(&leaf_node)?;
        Ok(Self { leaf_node_bytes })
    }

    pub(super) fn into_inner<LeafNode: LeafNodeTrait<CURRENT_VERSION>>(
        self,
    ) -> Result<LeafNode, serde_json::Error> {
        let leaf_node = serde_json::from_slice(&self.leaf_node_bytes)?;
        Ok(leaf_node)
    }

    pub(super) fn store<GroupId: GroupIdTrait<CURRENT_VERSION>>(
        &self,
        connection: &rusqlite::Connection,
        group_id: &GroupId,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "INSERT INTO own_leaf_nodes (group_id, leaf_node) VALUES (?1, ?2)",
            params![KeyRefWrapper(group_id), self.leaf_node_bytes],
        )?;
        Ok(())
    }

    pub(super) fn load<GroupId: GroupIdTrait<CURRENT_VERSION>>(
        connection: &rusqlite::Connection,
        group_id: &GroupId,
    ) -> Result<Vec<Self>, rusqlite::Error> {
        let mut stmt =
            connection.prepare("SELECT leaf_node FROM own_leaf_nodes WHERE group_id = ?")?;
        let leaf_nodes = stmt
            .query_map(params![KeyRefWrapper(group_id)], Self::from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(leaf_nodes)
    }

    pub(super) fn delete<GroupId: GroupIdTrait<CURRENT_VERSION>>(
        connection: &rusqlite::Connection,
        group_id: &GroupId,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "DELETE FROM own_leaf_nodes WHERE group_id = ?",
            params![KeyRefWrapper(group_id)],
        )?;
        Ok(())
    }
}
