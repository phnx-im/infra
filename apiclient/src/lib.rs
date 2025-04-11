// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! HTTP client for the server REST API

use std::{sync::Arc, time::Duration};

use ds_api::grpc::GrpcDsClient;
use phnxtypes::{DEFAULT_PORT_HTTP, DEFAULT_PORT_HTTPS, endpoint_paths::ENDPOINT_HEALTH_CHECK};
use protos::delivery_service::v1::delivery_service_client::DeliveryServiceClient;
use reqwest::{Client, ClientBuilder, StatusCode, Url};
use thiserror::Error;
use tonic::transport::Channel;
use tracing::info;
use url::ParseError;
use version::NegotiatedApiVersions;

pub mod as_api;
pub mod ds_api;
pub mod qs_api;
pub(crate) mod version;

/// Defines the type of protocol used for a specific endpoint.
pub enum Protocol {
    Http,
    Ws,
}

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

pub type HttpClient = reqwest::Client;

// ApiClient is a wrapper around a reqwest client.
// It exposes a single function for each API endpoint.
#[derive(Clone)]
pub struct ApiClient {
    client: HttpClient,
    grpc_ds_client: GrpcDsClient,
    url: Url,
    api_versions: Arc<NegotiatedApiVersions>,
}

impl ApiClient {
    /// Creates a new HTTP client.
    pub fn new_http_client() -> reqwest::Result<Client> {
        ClientBuilder::new()
            .pool_idle_timeout(Duration::from_secs(4))
            .user_agent("PhnxClient/0.1")
            .build()
    }

    pub fn with_default_http_client(domain: impl ToString) -> Result<Self, ApiClientInitError> {
        let client = Self::new_http_client();
        Self::initialize(client?, domain)
    }

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
    pub fn initialize(
        client: HttpClient,
        domain: impl ToString,
    ) -> Result<Self, ApiClientInitError> {
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

        // TODO: reuse http client
        let mut grpc_url = url.clone();
        grpc_url.set_port(Some(50051)).unwrap();

        let endpoint = Channel::from_shared(grpc_url.to_string())
            .map_err(|error| ApiClientInitError::UrlParsingError(error.to_string()))?;
        info!(%grpc_url, "connecting to gRPC");
        let channel = endpoint.connect_lazy();
        let ds_client = GrpcDsClient::new(DeliveryServiceClient::new(channel));

        Ok(Self {
            client,
            grpc_ds_client: ds_client,
            url,
            api_versions: Arc::new(NegotiatedApiVersions::new()),
        })
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
            protocol_str.push('s')
        };
        let url = format!(
            "{}://{}:{}{}",
            protocol_str,
            self.url.host_str().unwrap_or_default(),
            self.url.port().unwrap_or(if tls_enabled {
                DEFAULT_PORT_HTTPS
            } else {
                DEFAULT_PORT_HTTP
            }),
            endpoint
        );
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

    pub(crate) fn negotiated_versions(&self) -> &NegotiatedApiVersions {
        &self.api_versions
    }
}
