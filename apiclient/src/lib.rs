// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! API client for the phnx server.

use std::net::SocketAddr;

use http::StatusCode;
use phnxbackend::qs::Fqdn;
use phnxserver::endpoints::ENDPOINT_HEALTH_CHECK;
use reqwest::{Client, ClientBuilder};
use thiserror::Error;

pub mod as_api;
pub mod ds_api;
pub mod qs_api;

/// Defines whether transport encryption is enabled or not. Transport
/// encryption should be enabled when used in production and there is no load
/// balancer or reverse proxy in front of the server that terminates TLS
/// connections.
pub enum TransportEncryption {
    On,
    Off,
}

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
pub struct ApiClient {
    client: Client,
    domain_or_address: DomainOrAddress,
    transport_encryption: TransportEncryption,
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
    pub fn initialize(
        domain: impl Into<DomainOrAddress>,
        transport_encryption: TransportEncryption,
    ) -> Result<Self, ApiClientInitError> {
        let client = ClientBuilder::new().user_agent("PhnxClient/0.1").build()?;
        Ok(Self {
            client,
            domain_or_address: domain.into(),
            transport_encryption,
        })
    }

    /// Builds a URL for a given endpoint.
    fn build_url(&self, protocol: Protocol, endpoint: &str) -> String {
        let protocol = match protocol {
            Protocol::Http => "http",
            Protocol::Ws => "ws",
        };
        let transport_encryption = match self.transport_encryption {
            TransportEncryption::On => "s",
            TransportEncryption::Off => "",
        };
        let address_and_port = match &self.domain_or_address {
            DomainOrAddress::Domain(domain) => format!("{}:{}", domain, 8000),
            DomainOrAddress::Address(address) => address.to_string(),
        };
        format!(
            "{}{}://{}{}",
            protocol, transport_encryption, address_and_port, endpoint
        )
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

    pub fn domain_or_address(&self) -> &DomainOrAddress {
        &self.domain_or_address
    }
}
