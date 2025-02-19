// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::messages::client_qs::{
    ClientToQsMessageTbs, QsProcessResponseIn, QsVersionedProcessResponseIn, VersionError,
};

use super::QsRequestError;

pub(super) fn migrate_qs_process_response(
    response: QsVersionedProcessResponseIn,
) -> Result<QsProcessResponseIn, QsRequestError> {
    match response {
        QsVersionedProcessResponseIn::Alpha(response) => Ok(response),
        QsVersionedProcessResponseIn::Other(version) => Err(VersionError::new(
            version,
            ClientToQsMessageTbs::SUPPORTED_API_VERSIONS.to_vec(),
        )
        .into()),
    }
}
