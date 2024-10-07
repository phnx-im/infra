// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::Deserialize;

/// Configuration for the server.
#[derive(Deserialize, Clone, Debug)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
    // If this isn't present, the provider will not send push notifications to
    // apple devices.
    pub apns: Option<ApnsSettings>,
}

/// Configuration for the application.
#[derive(serde::Deserialize, Clone, Debug)]
pub struct ApplicationSettings {
    pub port: u16,
    pub host: String,
    pub domain: String,
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
pub struct ApnsSettings {
    pub keyid: String,
    pub teamid: String,
    pub privatekeypath: String,
}

impl DatabaseSettings {
    /// Add the TLS mode to the connection string if the CA certificate path is
    /// set.
    fn add_tls_mode(&self, mut connection_string: String) -> String {
        if let Some(ref ca_cert_path) = self.cacertpath {
            connection_string.push_str(&format!("?sslmode=verify-ca&sslrootcert={}", ca_cert_path));
        }
        connection_string
    }

    /// Get the connection string for the database.
    pub fn connection_string(&self) -> String {
        let connection_string = format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.name
        );
        self.add_tls_mode(connection_string)
    }

    /// Get the connection string for the database without the database name.
    pub fn connection_string_without_database(&self) -> String {
        let connection_string = format!(
            "postgres://{}:{}@{}:{}",
            self.username, self.password, self.host, self.port
        );
        self.add_tls_mode(connection_string)
    }
}
