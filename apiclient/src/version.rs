// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    collections::HashSet,
    sync::atomic::{AtomicU64, Ordering},
};

use http::{HeaderMap, StatusCode};
use phnxtypes::{
    messages::{client_qs::CURRENT_QS_API_VERSION, ApiVersion},
    ACCEPTED_API_VERSIONS_HEADER,
};
use tracing::error;

/// Api versions that were negotiated with the server.
///
/// The default values are the current API versions of the corresponding messages.
pub(crate) struct NegotiatedApiVersions {
    qs_api_version: AtomicU64,
}

impl NegotiatedApiVersions {
    pub(crate) fn new() -> Self {
        Self {
            qs_api_version: AtomicU64::new(CURRENT_QS_API_VERSION.value()),
        }
    }

    pub(crate) fn set_qs_api_version(&self, version: ApiVersion) {
        self.qs_api_version
            .store(version.value(), Ordering::Relaxed);
    }

    pub(crate) fn qs_api_version(&self) -> ApiVersion {
        let version = self.qs_api_version.load(Ordering::Relaxed);
        ApiVersion::new(version).expect("logic error")
    }
}

/// Returns Some if API version negotiation is required, otherwise None.
///
/// The returned set contains the supported API versions by the server.
pub(crate) fn extract_api_version_negotiation(
    response: &reqwest::Response,
) -> Option<HashSet<ApiVersion>> {
    if response.status() != StatusCode::NOT_ACCEPTABLE {
        return None;
    }
    parse_accepted_versions_header(response.headers())
}

fn parse_accepted_versions_header(headers: &HeaderMap) -> Option<HashSet<ApiVersion>> {
    let value = headers.get(ACCEPTED_API_VERSIONS_HEADER)?;
    let Ok(value) = value.to_str() else {
        error!(
            value =% String::from_utf8_lossy(value.as_bytes()),
            "Invalid value for {ACCEPTED_API_VERSIONS_HEADER} header"
        );
        return Some(Default::default());
    };
    let versions = value
        .split(',')
        .filter_map(|version| {
            version
                .trim()
                .parse()
                .inspect_err(|_| error!(version, "skipping invalid api version"))
                .ok()
        })
        .filter_map(ApiVersion::new)
        .collect();
    Some(versions)
}

/// Returns the highest API version that is supported by both the client and the server.
pub(crate) fn negotiate_api_version(
    accepted_versions: HashSet<ApiVersion>,
    supported_versions: &[ApiVersion],
) -> Option<ApiVersion> {
    supported_versions
        .iter()
        .copied()
        .filter(|version| accepted_versions.contains(version))
        .max()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_api_version_negotiation_some() {
        let response = http::response::Builder::new()
            .status(StatusCode::NOT_ACCEPTABLE)
            .header(ACCEPTED_API_VERSIONS_HEADER, "1,something,3")
            .body(Vec::new())
            .unwrap()
            .into();
        assert_eq!(
            extract_api_version_negotiation(&response),
            Some(
                [ApiVersion::new(1).unwrap(), ApiVersion::new(3).unwrap()]
                    .into_iter()
                    .collect()
            ),
        );
    }

    #[test]
    fn extract_api_version_negotiation_status_ok() {
        let response = http::response::Builder::new()
            .status(StatusCode::OK)
            .body(Vec::new())
            .unwrap()
            .into();
        assert!(extract_api_version_negotiation(&response).is_none());
    }

    #[test]
    fn extract_api_version_negotiation_header_missing() {
        let response = http::response::Builder::new()
            .status(StatusCode::NOT_ACCEPTABLE)
            .body(Vec::new())
            .unwrap()
            .into();
        assert!(extract_api_version_negotiation(&response).is_none());
    }

    #[test]
    fn negotiate_api_version_success() {
        let accepted_versions = [ApiVersion::new(1).unwrap(), ApiVersion::new(3).unwrap()]
            .into_iter()
            .collect();
        let supported_versions = &[ApiVersion::new(1).unwrap(), ApiVersion::new(2).unwrap()];
        assert_eq!(
            negotiate_api_version(accepted_versions, supported_versions),
            Some(ApiVersion::new(1).unwrap())
        );
    }

    #[test]
    fn negotiate_api_version_failure() {
        let accepted_versions = [ApiVersion::new(2).unwrap(), ApiVersion::new(4).unwrap()]
            .into_iter()
            .collect();
        let supported_versions = &[ApiVersion::new(1).unwrap(), ApiVersion::new(0).unwrap()];
        assert_eq!(
            negotiate_api_version(accepted_versions, supported_versions),
            None
        );
    }
}
