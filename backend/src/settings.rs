// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxcommon::DEFAULT_PORT_GRPC;
use serde::Deserialize;

/// Configuration for the server.
#[derive(Deserialize, Clone, Debug)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
    /// If this isn't present, the provider will not send push notifications to
    /// apple devices.
    pub apns: Option<ApnsSettings>,
    /// If this isn't present, the provider will not send push notifications to
    /// android devices.
    pub fcm: Option<FcmSettings>,
    /// If this isn't present, the support for attachments is disabled.
    pub storage: Option<StorageSettings>,
}

/// Configuration for the application.
#[derive(Deserialize, Clone, Debug)]
pub struct ApplicationSettings {
    pub port: u16,
    #[serde(default = "default_grpc_port")]
    pub grpc_port: u16,
    pub host: String,
    pub domain: String,
}

fn default_grpc_port() -> u16 {
    DEFAULT_PORT_GRPC
}

/// Configuration for the database.
#[derive(Deserialize, Clone, Debug)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: String,
    pub port: u16,
    pub host: String,
    pub name: String,
    pub cacertpath: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FcmSettings {
    // The path to the service account key file.
    pub path: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ApnsSettings {
    pub keyid: String,
    pub teamid: String,
    pub privatekeypath: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StorageSettings {
    /// Endpoint for the storage provider
    pub endpoint: String,
    /// Region for the storage provider
    pub region: String,
    /// Access key ID for the storage provider
    pub access_key_id: String,
    /// Secret access key for the storage provider
    pub secret_access_key: String,
    /// Force path style for the storage provider
    #[serde(default)]
    pub force_path_style: bool,
}

impl DatabaseSettings {
    /// Add the TLS mode to the connection string if the CA certificate path is
    /// set.
    fn add_tls_mode(&self, mut connection_string: String) -> String {
        if let Some(ref ca_cert_path) = self.cacertpath {
            connection_string.push_str(&format!("?sslmode=verify-ca&sslrootcert={}", ca_cert_path));
        } else {
            tracing::warn!(
                "No CA certificate path set for database connection. TLS will not be enabled."
            );
        }
        connection_string
    }

    /// Compose the base connection string without the database name.
    fn base_connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}",
            self.username, self.password, self.host, self.port
        )
    }

    /// Get the connection string for the database.
    pub fn connection_string(&self) -> String {
        let mut connection_string = self.base_connection_string();
        connection_string.push('/');
        connection_string.push_str(&self.name);
        self.add_tls_mode(connection_string)
    }

    /// Get the connection string for the database without the database name.
    /// Enables TLS by default.
    pub fn connection_string_without_database(&self) -> String {
        let connection_string = self.base_connection_string();
        self.add_tls_mode(connection_string)
    }
}
