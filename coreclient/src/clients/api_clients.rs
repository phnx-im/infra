// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    collections::{HashMap, hash_map::Entry},
    sync::Mutex,
};

use phnxcommon::identifiers::Fqdn;

use super::*;

#[derive(Debug, Clone)]
pub(crate) struct ApiClients {
    // We store our own domain such that we can manually map our own domain to
    // an API client that uses an IP address instead of the actual domain. This
    // is a temporary workaround and should probably be replaced by a more
    // thought-out mechanism.
    own_domain: Fqdn,
    own_domain_or_address: String,
    clients: Arc<Mutex<HashMap<String, ApiClient>>>,
    grpc_port: u16,
}

impl ApiClients {
    pub(super) fn new(
        own_domain: Fqdn,
        own_domain_or_address: impl ToString,
        grpc_port: u16,
    ) -> Self {
        let own_domain_or_address = own_domain_or_address.to_string();
        Self {
            own_domain,
            own_domain_or_address,
            clients: Default::default(),
            grpc_port,
        }
    }

    pub(crate) fn get(&self, domain: &Fqdn) -> Result<ApiClient, ApiClientsError> {
        let domain = if domain == &self.own_domain {
            self.own_domain_or_address.clone()
        } else {
            domain.to_string()
        };
        let mut clients = self.clients.lock().unwrap();
        let client = match clients.entry(domain) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => {
                let client = ApiClient::new(entry.key(), self.grpc_port)?;
                entry.insert(client).clone()
            }
        };
        Ok(client)
    }

    pub(super) fn default_client(&self) -> Result<ApiClient, ApiClientsError> {
        let own_domain = self.own_domain.clone();
        self.get(&own_domain)
    }
}

#[derive(Debug, Error)]
pub(crate) enum ApiClientsError {
    #[error(transparent)]
    ApiClientError(#[from] ApiClientInitError),
}
