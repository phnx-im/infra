// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use actix_web::web::{self, Data};
use phnxbackend::auth_service::{AuthService, VerifiableClientToAsMessage};
use phnxtypes::{
    ACCEPTED_API_VERSIONS_HEADER,
    errors::auth_service::{AsProcessingError, RegisterUserError},
};
use tls_codec::{DeserializeBytes, Serialize};
use tracing::{error, info, trace, warn};

use super::*;

/// DS endpoint for all group-based functionalities.
#[tracing::instrument(name = "Perform AS operation", skip_all)]
pub(crate) async fn as_process_message(
    message: web::Bytes,
    auth_service: Data<AuthService>,
) -> impl Responder {
    // Create a new group on the DS.
    let message = match VerifiableClientToAsMessage::tls_deserialize_exact_bytes(&message) {
        Ok(message) => message,
        Err(error) => {
            warn!(%error, "Received invalid message");
            return HttpResponse::BadRequest().body(error.to_string());
        }
    };
    match auth_service.process(message).await {
        // If the message was processed successfully, return the response.
        Ok(response) => {
            trace!("Processed message successfully");
            HttpResponse::Ok().body(response.tls_serialize_detached().unwrap())
        }
        Err(AsProcessingError::Api(version_error)) => {
            info!(%version_error, "Unsupported QS API version");
            HttpResponse::NotAcceptable()
                .insert_header((
                    ACCEPTED_API_VERSIONS_HEADER,
                    version_error.supported_versions_header_value(),
                ))
                .body(version_error.to_string())
        }
        Err(AsProcessingError::RegisterUserError(RegisterUserError::UserAlreadyExists)) => {
            HttpResponse::Conflict().body("User already exists")
        }
        // If the message could not be processed, return an error.
        Err(error) => {
            error!(%error, "AS failed to process message");
            HttpResponse::InternalServerError().body(error.to_string())
        }
    }
}
