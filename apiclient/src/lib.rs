// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! API client for the phnx server.

use std::{net::SocketAddr, time::Duration};

use http::StatusCode;
use phnxbackend::qs::Fqdn;
use phnxserver::endpoints::ENDPOINT_HEALTH_CHECK;
use reqwest::{Client, ClientBuilder, Url};
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
    #[error("Failed to parse URL {0}")]
    UrlParsingError(String),
    #[error("Could not find hostname in URL")]
    NoHostname,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
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
    port: u16,
    hostname: String,
    tls_enabled: bool,
}

impl ApiClient {
    /// Creates a new API client that connects to the given base URL.
    ///
    /// # Arguments
    /// url - The base URL or hostname:port tuple of the server. If the URL
    /// starts with `https`, TLS will be used. If the URL or hostname:port tuple
    /// includes a port, that port will be used, otherwise, if TLS is enabled,
    /// the default port is 443, if TLS is disabled, the default port is 8000.
    ///
    /// # Returns
    /// A new [`ApiClient`].
    pub fn initialize(domain: impl ToString) -> Result<Self, ApiClientInitError> {
        let mut domain_string = domain.to_string();
        // If the url doesn't start with http, we assume it's a hostname/port
        // combination and we prefix it with "http://"
        if !domain_string.starts_with("http") {
            domain_string.insert_str(0, "http://");
        }
        let url = Url::parse(domain_string.as_str())
            .map_err(|_| ApiClientInitError::UrlParsingError(domain_string.clone()))?;
        let tls_enabled = url.scheme() == "https";
        let port = url
            .port()
            .unwrap_or_else(|| if tls_enabled { 443 } else { 8000 });
        let hostname = url
            .host()
            .ok_or(ApiClientInitError::NoHostname)?
            .to_string();
        let client = ClientBuilder::new()
            .pool_idle_timeout(Duration::from_secs(4))
            .user_agent("PhnxClient/0.1")
            .build()?;
        Ok(Self {
            client,
            port,
            hostname,
            tls_enabled,
        })
    }

    /// Builds a URL for a given endpoint.
    fn build_url(&self, protocol: Protocol, endpoint: &str) -> String {
        let mut protocol = match protocol {
            Protocol::Http => "http",
            Protocol::Ws => "ws",
        }
        .to_string();
        if self.tls_enabled {
            protocol.push_str("s")
        };
        let url = format!("{}://{}:{}{}", protocol, self.hostname, self.port, endpoint);
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
}
