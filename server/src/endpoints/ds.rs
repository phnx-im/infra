// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use actix_web::{
    HttpResponse, Responder,
    web::{self, Data},
};
use phnxbackend::{ds::Ds, qs::QsConnector};
use phnxtypes::{ACCEPTED_API_VERSIONS_HEADER, errors::DsProcessingError};
use tls_codec::{DeserializeBytes, Serialize};
use tracing::{error, info, trace, warn};

/// DS endpoint for all group-based functionalities.
#[tracing::instrument(name = "Perform DS operation", skip_all)]
pub(crate) async fn ds_process_message<Qep: QsConnector>(
    message: web::Bytes,
    ds_storage_provider: Data<Ds>,
    qs_connector: Data<Qep>,
) -> impl Responder {
    // Extract the storage provider.
    let storage_provider = ds_storage_provider.get_ref();
    let qs_connector = qs_connector.get_ref();
    // Create a new group on the DS.
    let message = match DeserializeBytes::tls_deserialize_exact_bytes(&message) {
        Ok(message) => message,
        Err(error) => {
            warn!(%error, "Received invalid message");
            return HttpResponse::BadRequest().body(error.to_string());
        }
    };
    match Ds::process(storage_provider, qs_connector, message).await {
        // If the message was processed successfully, return the response.
        Ok(response) => {
            trace!("Processed message successfully");
            let serialized_response = response.tls_serialize_detached().unwrap();
            HttpResponse::Ok().body(serialized_response)
        }
        Err(DsProcessingError::Api(version_error)) => {
            info!(%version_error, "Unsupported QS API version");
            HttpResponse::NotAcceptable()
                .insert_header((
                    ACCEPTED_API_VERSIONS_HEADER,
                    version_error.supported_versions_header_value(),
                ))
                .body(version_error.to_string())
        }
        // If the message could not be processed, return an error.
        Err(error) => {
            error!(%error, "DS failed to process message");
            HttpResponse::InternalServerError().body(error.to_string())
        }
    }
}
