// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls_traits::storage::{
    traits::ProposalRef as ProposalRefTrait, Entity, Key, CURRENT_VERSION,
};
use rusqlite::params;

use crate::utils::persistence::Storable;

use super::storage_provider::{EntityRefWrapper, EntityWrapper, KeyRefWrapper, StorableGroupIdRef};

pub(crate) struct StorableProposal<
    Proposal: Entity<CURRENT_VERSION>,
    ProposalRef: Entity<CURRENT_VERSION>,
>(pub ProposalRef, pub Proposal);

impl<Proposal: Entity<CURRENT_VERSION>, ProposalRef: Entity<CURRENT_VERSION>> Storable
    for StorableProposal<Proposal, ProposalRef>
{
    const CREATE_TABLE_STATEMENT: &'static str = "CREATE TABLE IF NOT EXISTS proposals (
        group_id BLOB NOT NULL,
        proposal_ref BLOB NOT NULL,
        proposal BLOB NOT NULL,
        PRIMARY KEY (group_id, proposal_ref)
    );";

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        let EntityWrapper(proposal_ref) = row.get(0)?;
        let EntityWrapper(proposal) = row.get(1)?;
        Ok(Self(proposal_ref, proposal))
    }
}

impl<Proposal: Entity<CURRENT_VERSION>, ProposalRef: Entity<CURRENT_VERSION>>
    StorableProposal<Proposal, ProposalRef>
{
    pub(super) fn load<GroupId: Key<CURRENT_VERSION>>(
        connection: &rusqlite::Connection,
        group_id: &GroupId,
    ) -> Result<Vec<(ProposalRef, Proposal)>, rusqlite::Error> {
        let mut stmt = connection
            .prepare("SELECT proposal_ref, proposal FROM proposals WHERE group_id = ?1")?;
        let proposals = stmt
            .query_map(params![KeyRefWrapper(group_id)], |row| {
                Self::from_row(row).map(|x| (x.0, x.1))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(proposals)
    }

    pub(super) fn load_refs<GroupId: Key<CURRENT_VERSION>>(
        connection: &rusqlite::Connection,
        group_id: &GroupId,
    ) -> Result<Vec<ProposalRef>, rusqlite::Error> {
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
}

pub(super) struct StorableProposalRef<
    'a,
    Proposal: Entity<CURRENT_VERSION>,
    ProposalRef: Entity<CURRENT_VERSION>,
>(pub &'a ProposalRef, pub &'a Proposal);

impl<'a, Proposal: Entity<CURRENT_VERSION>, ProposalRef: Entity<CURRENT_VERSION>>
    StorableProposalRef<'a, Proposal, ProposalRef>
{
    pub(super) fn store<GroupId: Key<CURRENT_VERSION>>(
        &self,
        connection: &rusqlite::Connection,
        group_id: &GroupId,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "INSERT INTO proposals (group_id, proposal_ref, proposal) VALUES (?1, ?2, ?3)",
            params![
                KeyRefWrapper(group_id),
                EntityRefWrapper(self.0),
                EntityRefWrapper(self.1),
            ],
        )?;
        Ok(())
    }
}

impl<'a, GroupId: Key<CURRENT_VERSION>> StorableGroupIdRef<'a, GroupId> {
    pub(super) fn delete_all_proposals(
        &self,
        connection: &rusqlite::Connection,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "DELETE FROM proposals WHERE group_id = ?1",
            params![KeyRefWrapper(self.0)],
        )?;
        Ok(())
    }

    pub(super) fn delete_proposal<ProposalRef: ProposalRefTrait<CURRENT_VERSION>>(
        &self,
        connection: &rusqlite::Connection,
        proposal_ref: &ProposalRef,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "DELETE FROM proposals WHERE group_id = ?1 AND proposal_ref = ?2",
            params![KeyRefWrapper(self.0), KeyRefWrapper(proposal_ref)],
        )?;
        Ok(())
    }
}
