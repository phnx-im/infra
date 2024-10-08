// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use actix_web::web::{self, Data};
use phnxbackend::auth_service::{AuthService, VerifiableClientToAsMessage};
use tls_codec::{DeserializeBytes, Serialize};

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
        Err(e) => {
            tracing::warn!("Received invalid message: {:?}", e);
            return HttpResponse::BadRequest().body(e.to_string());
        }
    };
    match auth_service.process(message).await {
        // If the message was processed successfully, return the response.
        Ok(response) => {
            tracing::trace!("Processed message successfully");
            HttpResponse::Ok().body(response.tls_serialize_detached().unwrap())
        }
        // If the message could not be processed, return an error.
        Err(e) => {
            tracing::warn!("AS failed to process message: {:?}", e);
            HttpResponse::InternalServerError().body(e.to_string())
        }
    }
}
