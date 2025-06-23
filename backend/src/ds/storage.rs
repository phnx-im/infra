// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use aws_config::Region;
use aws_sdk_s3::{Client, Config, config::Credentials};

use crate::settings::StorageSettings;

#[derive(Debug, Clone)]
pub struct Storage {
    pub(crate) config: Config,
}

impl Storage {
    pub fn new(settings: StorageSettings) -> Self {
        let credentials = Credentials::new(
            settings.access_key_id,
            settings.secret_access_key,
            None,
            None,
            "storage",
        );
        let config = Config::builder()
            .endpoint_url(settings.endpoint)
            .region(Region::new(settings.region))
            .credentials_provider(credentials)
            .force_path_style(settings.force_path_style)
            .behavior_version_latest()
            .build();
        Self { config }
    }

    pub(crate) fn client(&self) -> Client {
        Client::from_conf(self.config.clone())
    }
}
