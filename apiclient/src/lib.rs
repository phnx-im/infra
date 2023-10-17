// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! API client for the phnx server.

use std::time::Duration;

use http::StatusCode;
use phnxtypes::endpoint_paths::ENDPOINT_HEALTH_CHECK;
use reqwest::{Client, ClientBuilder, Url};
use thiserror::Error;
use url::ParseError;

pub mod as_api;
pub mod ds_api;
pub mod qs_api;

/// Defines the type of protocol used for a specific endpoint.
pub enum Protocol {
    Http,
    Ws,
}

pub const DEFAULT_PORT_HTTP: u16 = 80;
pub const DEFAULT_PORT_HTTPS: u16 = 443;
// TODO: Turn this on once we have the necessary test infrastructure for
// certificates in place.
const HTTPS_BY_DEFAULT: bool = false;

#[derive(Error, Debug)]
pub enum ApiClientInitError {
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error("Failed to parse URL {0}")]
    UrlParsingError(String),
    #[error("Could not find hostname in URL {0}")]
    NoHostname(String),
    #[error("The use of TLS is mandatory")]
    TlsRequired,
}

// ApiClient is a wrapper around a reqwest client.
// It exposes a single function for each API endpoint.
#[derive(Clone)]
pub struct ApiClient {
    client: Client,
    url: Url,
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
        // We first check if the domain is a valid URL.
        let url = match Url::parse(&domain_string) {
            Ok(url) => url,
            // If not, we try to parse it as a hostname.
            Err(ParseError::RelativeUrlWithoutBase) => {
                let protocol_str = if HTTPS_BY_DEFAULT { "https" } else { "http" };
                domain_string = format!("{}://{}", protocol_str, domain_string);
                Url::parse(&domain_string)
                    .map_err(|_| ApiClientInitError::UrlParsingError(domain_string.clone()))?
            }
            Err(_) => return Err(ApiClientInitError::UrlParsingError(domain_string.clone())),
        };
        let client = ClientBuilder::new()
            .pool_idle_timeout(Duration::from_secs(4))
            .user_agent("PhnxClient/0.1")
            .build()?;
        Ok(Self { client, url })
    }

    /// Builds a URL for a given endpoint.
    fn build_url(&self, protocol: Protocol, endpoint: &str) -> String {
        let mut protocol_str = match protocol {
            Protocol::Http => "http",
            Protocol::Ws => "ws",
        }
        .to_string();
        let tls_enabled = self.url.scheme() == "https";
        if tls_enabled {
            protocol_str.push_str("s")
        };
        let url = format!(
            "{}://{}:{}{}",
            protocol_str,
            self.url.host_str().unwrap_or_default(),
            self.url.port().unwrap_or_else(|| if tls_enabled {
                DEFAULT_PORT_HTTPS
            } else {
                DEFAULT_PORT_HTTP
            }),
            endpoint
        );
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
