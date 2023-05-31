// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! API client for the phnx server.

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

// ApiClient is a wrapper around a reqwest client.
// It exposes a single function for each API endpoint.
pub struct ApiClient {
    client: Client,
    base_url: String,
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
        base_url: String,
        transport_encryption: TransportEncryption,
    ) -> Result<Self, ApiClientInitError> {
        let client = ClientBuilder::new().user_agent("PhnxClient/0.1").build()?;
        Ok(Self {
            client,
            base_url,
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
        format!(
            "{}{}://{}{}",
            protocol, transport_encryption, self.base_url, endpoint
        )
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}
