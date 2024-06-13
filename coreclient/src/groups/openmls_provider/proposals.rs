// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls_traits::storage::{
    traits::{
        GroupId as GroupIdTrait, ProposalRef as ProposalRefTrait, QueuedProposal as ProposalTrait,
    },
    CURRENT_VERSION,
};
use rusqlite::params;

use crate::utils::persistence::Storable;

use super::storage_provider::{EntityWrapper, KeyRefWrapper, SqliteStorageProviderError};

pub(crate) struct StorableProposal {
    proposal_ref_bytes: Vec<u8>,
    proposal_bytes: Vec<u8>,
}

impl Storable for StorableProposal {
    const CREATE_TABLE_STATEMENT: &'static str = "CREATE TABLE IF NOT EXISTS proposals (
        group_id BLOB NOT NULL,
        proposal_ref BLOB NOT NULL,
        proposal BLOB NOT NULL,
        PRIMARY KEY (group_id, proposal_ref)
    )";

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        let proposal_ref_bytes = row.get(0)?;
        let proposal_bytes = row.get(1)?;
        Ok(Self {
            proposal_ref_bytes,
            proposal_bytes,
        })
    }
}

impl StorableProposal {
    pub(super) fn new<
        ProposalRef: ProposalRefTrait<CURRENT_VERSION>,
        Proposal: ProposalTrait<CURRENT_VERSION>,
    >(
        proposal_ref: &ProposalRef,
        proposal: &Proposal,
    ) -> Result<Self, serde_json::Error> {
        let proposal_ref_bytes = serde_json::to_vec(&proposal_ref)?;
        let proposal_bytes = serde_json::to_vec(proposal)?;
        Ok(Self {
            proposal_ref_bytes,
            proposal_bytes,
        })
    }

    pub(super) fn into_tuple<
        ProposalRef: ProposalRefTrait<CURRENT_VERSION>,
        Proposal: ProposalTrait<CURRENT_VERSION>,
    >(
        self,
    ) -> Result<(ProposalRef, Proposal), serde_json::Error> {
        let proposal_ref = serde_json::from_slice(&self.proposal_ref_bytes)?;
        let proposal = serde_json::from_slice(&self.proposal_bytes)?;
        Ok((proposal_ref, proposal))
    }

    pub(super) fn store<GroupId: GroupIdTrait<CURRENT_VERSION>>(
        &self,
        connection: &rusqlite::Connection,
        group_id: &GroupId,
    ) -> Result<(), SqliteStorageProviderError> {
        connection.execute(
            "INSERT INTO proposals (group_id, proposal_ref, proposal) VALUES (?1, ?2, ?3)",
            params![
                KeyRefWrapper(group_id),
                self.proposal_ref_bytes,
                self.proposal_bytes
            ],
        )?;
        Ok(())
    }

    pub(super) fn load<GroupId: GroupIdTrait<CURRENT_VERSION>>(
        connection: &rusqlite::Connection,
        group_id: &GroupId,
    ) -> Result<Vec<Self>, SqliteStorageProviderError> {
        let mut stmt = connection
            .prepare("SELECT proposal_ref, proposal FROM proposals WHERE group_id = ?1")?;
        let proposals = stmt
            .query_map(params![KeyRefWrapper(group_id)], Self::from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(proposals)
    }

    pub(super) fn load_refs<
        GroupId: GroupIdTrait<CURRENT_VERSION>,
        ProposalRef: ProposalRefTrait<CURRENT_VERSION>,
    >(
        connection: &rusqlite::Connection,
        group_id: &GroupId,
    ) -> Result<Vec<ProposalRef>, SqliteStorageProviderError> {
        let mut stmt =
            connection.prepare("SELECT proposal_ref FROM proposals WHERE group_id = ?1")?;
        let proposal_refs = stmt
            .query_map(params![KeyRefWrapper(group_id)], |row| {
                let EntityWrapper(proposal_ref) = row.get(0)?;
                Ok(proposal_ref)
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(proposal_refs)
    }

    pub(super) fn delete_all<GroupId: GroupIdTrait<CURRENT_VERSION>>(
        connection: &rusqlite::Connection,
        group_id: &GroupId,
    ) -> Result<(), SqliteStorageProviderError> {
        connection.execute(
            "DELETE FROM proposals WHERE group_id = ?1",
            params![KeyRefWrapper(group_id)],
        )?;
        Ok(())
    }

    pub(super) fn delete<
        GroupId: GroupIdTrait<CURRENT_VERSION>,
        ProposalRef: ProposalRefTrait<CURRENT_VERSION>,
    >(
        connection: &rusqlite::Connection,
        group_id: &GroupId,
        proposal_ref: &ProposalRef,
    ) -> Result<(), SqliteStorageProviderError> {
        connection.execute(
            "DELETE FROM proposals WHERE group_id = ?1 AND proposal_ref = ?2",
            params![KeyRefWrapper(group_id), KeyRefWrapper(proposal_ref)],
        )?;
        Ok(())
    }
}
