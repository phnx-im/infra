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
            Err(VersionError::from_unsupported_version(version).into())
        }
    }
}
