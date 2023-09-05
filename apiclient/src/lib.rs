// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! API client for the phnx server.

use std::{net::SocketAddr, time::Duration};

use http::StatusCode;
use phnxbackend::qs::Fqdn;
use phnxserver::endpoints::ENDPOINT_HEALTH_CHECK;
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod as_api;
pub mod ds_api;
pub mod qs_api;

/// Defines the type of protocol used for a specific endpoint.
pub enum Protocol {
    Http,
    Ws,
}

#[derive(Error, Debug)]
pub enum ApiClientInitError {
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum DomainOrAddress {
    Domain(Fqdn),
    Address(SocketAddr),
}

impl From<Fqdn> for DomainOrAddress {
    fn from(domain: Fqdn) -> Self {
        Self::Domain(domain)
    }
}

impl From<SocketAddr> for DomainOrAddress {
    fn from(address: SocketAddr) -> Self {
        Self::Address(address)
    }
}

impl std::fmt::Display for DomainOrAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DomainOrAddress::Domain(domain) => write!(f, "{}", domain),
            DomainOrAddress::Address(address) => write!(f, "{}", address),
        }
    }
}

// ApiClient is a wrapper around a reqwest client.
// It exposes a single function for each API endpoint.
#[derive(Clone)]
pub struct ApiClient {
    client: Client,
    url: String,
}

impl ApiClient {
    /// Creates a new API client that connects to the given base URL.
    ///
    /// # Arguments
    /// base_url - The base URL of the server.
    /// transport_encryption - Whether transport encryption is enabled or not.
    ///
    /// # Returns
    /// A new [`ApiClient`].
    pub fn initialize(domain: impl ToString) -> Result<Self, ApiClientInitError> {
        let client = ClientBuilder::new()
            .pool_idle_timeout(Duration::from_secs(4))
            .user_agent("PhnxClient/0.1")
            .build()?;
        Ok(Self {
            client,
            url: domain.to_string(),
        })
    }

    /// Builds a URL for a given endpoint.
    fn build_url(&self, protocol: Protocol, endpoint: &str) -> String {
        let tls = self.url.starts_with("https");
        let protocol = match protocol {
            Protocol::Http => "http",
            Protocol::Ws => "ws",
        };
        let protocol = if tls {
            format!("{}s", protocol)
        } else {
            protocol.to_string()
        };
        let domain_and_port = self.url.split("://").last().unwrap_or("");
        let url = format!("{}://{}{}", protocol, domain_and_port, endpoint);
        log::info!("Built URL: {}", url);
        url
    }

    /// Call the health check endpoint
    pub async fn health_check(&self) -> bool {
        self.client
            .get(self.build_url(Protocol::Http, ENDPOINT_HEALTH_CHECK))
            .send()
            .await
            .is_ok()
    }

    /// Call an inexistant endpoint
    pub async fn inexistant_endpoint(&self) -> bool {
        let res = self
            .client
            .post(self.build_url(Protocol::Http, "/null"))
            .body("test")
            .send()
            .await;
        let status = match res {
            Ok(r) => Some(r.status()),
            Err(e) => e.status(),
        };
        if let Some(status) = status {
            status == StatusCode::NOT_FOUND
        } else {
            false
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}
