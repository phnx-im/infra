// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use config::{Config, ConfigError};
use phnxbackend::settings::Settings;

/// The possible runtime environment for our application.
pub enum Environment {
    Local,
    Production,
}

impl Environment {
    pub fn from_env() -> Result<Self, String> {
        std::env::var("APP_ENVIRONMENT")
            .unwrap_or_else(|_| "local".into())
            .try_into()
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{} is not a supported environment. Use either `local` or `production`.",
                other
            )),
        }
    }
}

/// Load the configuration from the configuration file.
pub fn get_configuration(prefix: &str) -> Result<Settings, ConfigError> {
    // Directories
    let base_path = std::env::current_dir().expect("Failed to determine the current directory");
    let configuration_directory = base_path.join(format!("{}configuration", prefix));

    // Detect the running environment.
    // Default to `local` if unspecified.
    let environment = Environment::from_env().map_err(ConfigError::Message)?;

    let builder = Config::builder()
        // Read the "default" configuration file
        .add_source(config::File::from(configuration_directory.join("base")).required(true))
        // Layer on the environment-specific values.
        .add_source(
            config::File::from(configuration_directory.join(environment.as_str())).required(true),
        )
        // Add in settings from environment variables (with a prefix of APP and '_' as separator)
        // E.g. `PHNX_APPLICATION_PORT=5001 would set `Settings.application.port`
        .add_source(config::Environment::with_prefix("PHNX").separator("_"));

    builder.build()?.try_deserialize::<Settings>()
}
