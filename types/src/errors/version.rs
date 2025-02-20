// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::messages::ApiVersion;

#[derive(Debug, thiserror::Error)]
#[error("Unsupported version: {version}, supported versions: {supported_versions:?}")]
pub struct VersionError {
    version: ApiVersion,
    supported_versions: &'static [ApiVersion],
}

impl VersionError {
    pub fn new(version: ApiVersion, supported_versions: &'static [ApiVersion]) -> Self {
        Self {
            version,
            supported_versions,
        }
    }

    pub fn supported_versions(&self) -> &[ApiVersion] {
        self.supported_versions
    }

    pub fn supported_versions_header_value(&self) -> String {
        self.supported_versions
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(",")
    }

    pub fn parse_supported_versions_header_value(
        value: &str,
    ) -> impl Iterator<Item = ApiVersion> + '_ {
        value
            .split(',')
            .filter_map(|s| s.parse().ok())
            .filter_map(ApiVersion::new)
    }
}
