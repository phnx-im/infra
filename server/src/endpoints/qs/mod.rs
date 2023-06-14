// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use actix_web::{
    web::{self, Data},
    HttpResponse, Responder,
};
use phnxbackend::{
    messages::client_qs::VerifiableClientToQsMessage,
    qs::{storage_provider_trait::QsStorageProvider, Qs},
};
use tls_codec::{DeserializeBytes, Serialize};

pub mod ws;

/// QS endpoint to fetch queue config encryption key
#[utoipa::path(
    get,
    path = "{QS_ENDPOINT}",
    tag = "QS",
    responses(
        (status = 200, description = "Processed QS request."),
    )
)]
#[tracing::instrument(name = "Process QS message", skip_all)]
pub(crate) async fn qs_process_message<Qsp: QsStorageProvider>(
    qs_storage_provider: Data<Qsp>,
    message: web::Bytes,
) -> impl Responder {
    // Extract the storage provider.
    let storage_provider = qs_storage_provider.get_ref();

    // Deserialize the message.
    let message = match VerifiableClientToQsMessage::tls_deserialize_exact(message.as_ref()) {
        Ok(message) => message,
        Err(e) => {
            tracing::warn!("Received invalid message: {:?}", e);
            return HttpResponse::BadRequest().body(e.to_string());
        }
    };
    // Process the message.
    match Qs::process(storage_provider, message).await {
        // If the message was processed successfully, return the response.
        Ok(response) => {
            tracing::trace!("Processed message successfully");
            HttpResponse::Ok().body(response.tls_serialize_detached().unwrap())
        }
        // If the message could not be processed, return an error.
        Err(e) => {
            tracing::warn!("QS failed to process message: {:?}", e);
            HttpResponse::InternalServerError().body(e.to_string())
        }
    }
}
