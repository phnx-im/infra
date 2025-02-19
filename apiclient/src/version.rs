// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    collections::HashSet,
    sync::atomic::{AtomicU64, Ordering},
};

use http::{HeaderMap, StatusCode};
use phnxtypes::{
    messages::{
        client_qs::{ClientToQsMessageTbs, VersionError},
        ApiVersion,
    },
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
            qs_api_version: AtomicU64::new(ClientToQsMessageTbs::CURRENT_API_VERSION.value()),
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

/// Returns `Some` if the server supports a different API version, otherwise None.
///
/// If there is no API version supported by this client which is accepted by the server, the
/// returned result is an error.
pub(crate) fn api_version_negotiation(
    response: &reqwest::Response,
    current_version: ApiVersion,
    supported_versions: &[ApiVersion],
) -> Option<Result<ApiVersion, VersionError>> {
    if response.status() != StatusCode::NOT_ACCEPTABLE {
        return None;
    }

    let accepted_versions = parse_accepted_versions_header(response.headers())?;

    let accepted_version = negotiate_version(
        accepted_versions,
        supported_versions.iter().copied().collect(),
    );
    let accepted_version = accepted_version
        .ok_or_else(|| VersionError::new(current_version, supported_versions.to_vec()));

    Some(accepted_version)
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
fn negotiate_version(
    accepted_versions: HashSet<ApiVersion>,
    supported_versions: HashSet<ApiVersion>,
) -> Option<ApiVersion> {
    accepted_versions
        .intersection(&supported_versions)
        .cloned()
        .max()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_version_negotiation_needed() {
        let response = http::response::Builder::new()
            .status(StatusCode::NOT_ACCEPTABLE)
            .header(ACCEPTED_API_VERSIONS_HEADER, "1,something,3")
            .body(Vec::new())
            .unwrap()
            .into();

        let v1 = ApiVersion::new(1).unwrap();
        let v2 = ApiVersion::new(2).unwrap();
        let v3 = ApiVersion::new(3).unwrap();
        let v4 = ApiVersion::new(4).unwrap();

        let current_version = v1;
        assert_eq!(
            api_version_negotiation(&response, current_version, &[v1])
                .transpose()
                .unwrap(),
            Some(v1)
        );
        assert_eq!(
            api_version_negotiation(&response, current_version, &[v1, v3])
                .transpose()
                .unwrap(),
            Some(v3)
        );
        assert_eq!(
            api_version_negotiation(&response, current_version, &[v1, v2, v3, v4])
                .transpose()
                .unwrap(),
            Some(v3)
        );
        assert!(
            api_version_negotiation(&response, current_version, &[v2, v4])
                .transpose()
                .is_err()
        );
    }

    #[test]
    fn api_version_negotiation_not_needed() {
        let response = http::response::Builder::new()
            .status(StatusCode::OK)
            .body(Vec::new())
            .unwrap()
            .into();

        let v1 = ApiVersion::new(1).unwrap();
        assert!(api_version_negotiation(&response, v1, &[v1]).is_none());
    }

    #[test]
    fn api_version_negotiation_header_missing() {
        let response = http::response::Builder::new()
            .status(StatusCode::NOT_ACCEPTABLE)
            .body(Vec::new())
            .unwrap()
            .into();

        let v1 = ApiVersion::new(1).unwrap();
        assert!(api_version_negotiation(&response, v1, &[v1]).is_none());
    }
}
