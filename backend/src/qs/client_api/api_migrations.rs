// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::messages::client_qs::{
    ClientToQsMessageTbs, QsRequestParams, QsVersionedRequestParams, VersionError,
};

/// Migrates the given `params` to the latest version supported by the server.
pub(crate) fn migrate_qs_request_params(
    params: QsVersionedRequestParams,
) -> Result<QsRequestParams, VersionError> {
    match params {
        QsVersionedRequestParams::Alpha(params) => Ok(params),
        QsVersionedRequestParams::Other(version) => Err(VersionError::new(
            version,
            ClientToQsMessageTbs::SUPPORTED_API_VERSIONS.to_vec(),
        )),
    }
}
