use phnxtypes::messages::client_qs::{QsRequestParams, QsVersionedRequestParams, VersionError};

/// Migrates the given `params` to the latest version supported by the server.
pub(crate) fn migrate_qs_request_params(
    params: QsVersionedRequestParams,
) -> Result<QsRequestParams, VersionError> {
    match params {
        QsVersionedRequestParams::Alpha(params) => Ok(params),
        QsVersionedRequestParams::Other(version) => {
            Err(VersionError::from_unsupported_version(version))
        }
    }
}
