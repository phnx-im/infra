// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnx_types::identifiers::Fqdn;

use super::*;

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct ApiClients {
    // We store our own domain such that we can manually map our own domain to
    // an API client that uses an IP address instead of the actual domain. This
    // is a temporary workaround and should probably be replaced by a more
    // thought-out mechanism.
    own_domain: Fqdn,
    own_domain_or_address: String,
    #[serde(skip)]
    clients: Arc<Mutex<HashMap<String, ApiClient>>>,
}

impl ApiClients {
    pub(super) fn new(own_domain: Fqdn, own_domain_or_address: impl ToString) -> Self {
        let own_domain_or_address = own_domain_or_address.to_string();
        Self {
            own_domain,
            own_domain_or_address,
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub(crate) fn get(&self, domain: &Fqdn) -> Result<ApiClient, ApiClientsError> {
        let lookup_domain = if domain == &self.own_domain {
            self.own_domain_or_address.clone()
        } else {
            domain.clone().to_string()
        };
        let mut clients = self
            .clients
            .lock()
            .map_err(|_| ApiClientsError::MutexPoisonError)?;
        let client = clients
            .entry(lookup_domain.clone())
            .or_insert(ApiClient::initialize(lookup_domain)?);
        Ok(client.clone())
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
    #[error("Mutex poisoned")]
    MutexPoisonError,
}
