// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::messages::client_qs::{
    QsProcessResponseIn, QsVersionedProcessResponseIn, VersionError,
};

use super::QsRequestError;

pub(super) fn migrate_qs_process_response(
    response: QsVersionedProcessResponseIn,
) -> Result<QsProcessResponseIn, QsRequestError> {
    match response {
        QsVersionedProcessResponseIn::Alpha(response) => Ok(response),
        QsVersionedProcessResponseIn::Other(version) => {
            Err(VersionError::from_unsupported_version(version.into()).into())
        }
    }
}
