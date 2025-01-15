// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxapiclient::HttpClient;
use phnxtypes::identifiers::Fqdn;

use super::*;

#[derive(Clone)]
pub(crate) struct ApiClients {
    // We store our own domain such that we can manually map our own domain to
    // an API client that uses an IP address instead of the actual domain. This
    // is a temporary workaround and should probably be replaced by a more
    // thought-out mechanism.
    own_domain: Fqdn,
    own_domain_or_address: String,
    http_client: HttpClient,
}

impl ApiClients {
    pub(super) fn new(own_domain: Fqdn, own_domain_or_address: impl ToString) -> Self {
        let own_domain_or_address = own_domain_or_address.to_string();
        Self {
            own_domain,
            own_domain_or_address,
            http_client: ApiClient::new_http_client().expect("failed to initialize HTTP client"),
        }
    }

    pub(crate) fn get(&self, domain: &Fqdn) -> Result<ApiClient, ApiClientsError> {
        let domain = if domain == &self.own_domain {
            self.own_domain_or_address.clone()
        } else {
            domain.to_string()
        };
        Ok(ApiClient::initialize(self.http_client.clone(), domain)?)
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
