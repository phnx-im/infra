// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! HTTP client for the server REST API

use std::{sync::Arc, time::Duration};

use phnxtypes::{DEFAULT_PORT_HTTP, DEFAULT_PORT_HTTPS, endpoint_paths::ENDPOINT_HEALTH_CHECK};
use reqwest::{Certificate, Client, ClientBuilder, StatusCode, Url};
use rustls::{
    SignatureScheme,
    client::danger::{HandshakeSignatureValid, ServerCertVerified},
};
use thiserror::Error;
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
    url: Url,
    api_versions: Arc<NegotiatedApiVersions>,
}

#[derive(Debug)]
struct NoVerifier;

impl rustls::client::danger::ServerCertVerifier for NoVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA1,
            SignatureScheme::ECDSA_SHA1_Legacy,
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP521_SHA512,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ED25519,
            SignatureScheme::ED448,
        ]
    }
}

impl ApiClient {
    /// Creates a new HTTP client.
    pub fn new_http_client() -> reqwest::Result<Client> {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let root_store = rustls::RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
        };
        let mut tls = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        tls.key_log = Arc::new(rustls::KeyLogFile::new());
        tls.alpn_protocols = vec!["h2".into()];

        tls.dangerous()
            .set_certificate_verifier(Arc::new(NoVerifier));
        ClientBuilder::new()
            .use_preconfigured_tls(tls)
            .pool_idle_timeout(Duration::from_secs(4))
            .user_agent("PhnxClient/0.1")
            .danger_accept_invalid_certs(true)
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
        let url = "https://localhost:9420".parse().unwrap();
        Ok(Self {
            client,
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
