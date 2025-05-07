// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! HTTP client for the server REST API

use std::time::Duration;

use as_api::grpc::AsGrpcClient;
use ds_api::grpc::DsGrpcClient;
use phnxprotos::{
    auth_service::v1::auth_service_client::AuthServiceClient,
    delivery_service::v1::delivery_service_client::DeliveryServiceClient,
    queue_service::v1::queue_service_client::QueueServiceClient,
};
use qs_api::grpc::QsGrpcClient;
use reqwest::Url;
use thiserror::Error;
use tonic::transport::ClientTlsConfig;
use tracing::info;
use url::ParseError;

pub mod as_api;
pub mod ds_api;
pub mod qs_api;

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
    #[error("Invalid URL {0}")]
    InvalidUrl(String),
    #[error("Could not find hostname in URL {0}")]
    NoHostname(String),
    #[error("The use of TLS is mandatory")]
    TlsRequired,
    #[error(transparent)]
    TonicTranspor(#[from] tonic::transport::Error),
}

pub type HttpClient = reqwest::Client;

// ApiClient is a wrapper around a reqwest client.
// It exposes a single function for each API endpoint.
#[derive(Clone)]
pub struct ApiClient {
    as_grpc_client: AsGrpcClient,
    qs_grpc_client: QsGrpcClient,
    ds_grpc_client: DsGrpcClient,
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
    pub fn new(domain: impl AsRef<str>, grpc_port: u16) -> Result<Self, ApiClientInitError> {
        // We first check if the domain is a valid URL.
        let domain = domain.as_ref();
        let mut url = match Url::parse(domain) {
            Ok(url) => url,
            // If not, we try to parse it as a hostname.
            Err(ParseError::RelativeUrlWithoutBase) => {
                let protocol = if HTTPS_BY_DEFAULT { "https" } else { "http" };
                let domain = format!("{protocol}://{domain}");
                Url::parse(&domain).map_err(|_| ApiClientInitError::UrlParsingError(domain))?
            }
            Err(_) => return Err(ApiClientInitError::UrlParsingError(domain.to_owned())),
        };

        // For now, we are running grpc on the same domain but under a different port.
        url.set_port(Some(grpc_port)).expect("invalid url");
        info!(%url, "Connecting lazily to GRPC server");
        // TODO: Reuse HTTP client here
        let endpoint = tonic::transport::Endpoint::from_shared(url.to_string())
            .map_err(|_| ApiClientInitError::InvalidUrl(url.to_string()))?;
        let channel = endpoint
            .tls_config(ClientTlsConfig::new().with_webpki_roots())?
            .http2_keep_alive_interval(Duration::from_secs(30))
            .connect_lazy();
        let as_grpc_client = AsGrpcClient::new(AuthServiceClient::new(channel.clone()));
        let ds_grpc_client = DsGrpcClient::new(DeliveryServiceClient::new(channel.clone()));
        let qs_grpc_client = QsGrpcClient::new(QueueServiceClient::new(channel));

        Ok(Self {
            as_grpc_client,
            qs_grpc_client,
            ds_grpc_client,
        })
    }
}
