// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{collections::HashSet, sync::Arc};

use phnxtypes::{identifiers::Fqdn, time::Duration};
use sqlx::PgPool;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::infra_service::{InfraService, ServiceCreationError};

mod delete_group;
mod group_operation;
pub mod group_state;
mod grpc;
mod join_connection_group;
pub mod process;
mod resync;
mod self_remove;
mod update;

/// Number of days after its last use upon which a group state is considered
/// expired.
pub const GROUP_STATE_EXPIRATION: Duration = Duration::days(90);

#[derive(Debug, Clone)]
pub struct Ds {
    own_domain: Fqdn,
    reserved_group_ids: Arc<Mutex<HashSet<Uuid>>>,
    db_pool: PgPool,
}

#[derive(Debug)]
pub struct ReservedGroupId(Uuid);

impl InfraService for Ds {
    async fn initialize(db_pool: PgPool, domain: Fqdn) -> Result<Self, ServiceCreationError> {
        let ds = Self {
            own_domain: domain,
            reserved_group_ids: Default::default(),
            db_pool,
        };

        Ok(ds)
    }
}

impl Ds {
    async fn reserve_group_id(&self, group_id: Uuid) -> bool {
        let mut reserved_group_ids = self.reserved_group_ids.lock().await;
        reserved_group_ids.insert(group_id)
    }

    async fn claim_reserved_group_id(&self, group_id: Uuid) -> Option<ReservedGroupId> {
        let mut reserved_group_ids = self.reserved_group_ids.lock().await;
        if reserved_group_ids.remove(&group_id) {
            Some(ReservedGroupId(group_id))
        } else {
            None
        }
    }

    pub fn own_domain(&self) -> &Fqdn {
        &self.own_domain
    }

    pub fn pool(&self) -> &PgPool {
        &self.db_pool
    }
}
