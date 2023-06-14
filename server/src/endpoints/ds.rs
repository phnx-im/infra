// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use actix_web::{
    web::{self, Data},
    HttpResponse, Responder,
};
use phnxbackend::{
    ds::{api::DsApi, DsStorageProvider},
    messages::client_ds::VerifiableClientToDsMessage,
    qs::QsConnector,
};
use tls_codec::Serialize;

/// DS endpoint for all functionalities.
#[utoipa::path(
    post,
    path = "{ENDPOINT_DS}",
    tag = "DS",
    request_body = VerifiableClientToDsMessage,
    responses(
        (status = 200, description = "Group created successfully."),
    )
)]
#[tracing::instrument(name = "Perform DS operation", skip_all)]
pub(crate) async fn ds_process_message<Dsp: DsStorageProvider, Qep: QsConnector>(
    message: web::Bytes,
    ds_storage_provider: Data<Dsp>,
    qs_connector: Data<Qep>,
) -> impl Responder {
    // Extract the storage provider.
    let storage_provider = ds_storage_provider.get_ref();
    let qs_connector = qs_connector.get_ref();
    // Create a new group on the DS.
    let message = match VerifiableClientToDsMessage::try_from_bytes(&message) {
        Ok(message) => message,
        Err(e) => {
            tracing::warn!("Received invalid message: {:?}", e);
            return HttpResponse::BadRequest().body(e.to_string());
        }
    };
    match DsApi::process(storage_provider, qs_connector, message).await {
        // If the message was processed successfully, return the response.
        Ok(response) => {
            tracing::trace!("Processed message successfully");
            HttpResponse::Ok().body(response.tls_serialize_detached().unwrap())
        }
        // If the message could not be processed, return an error.
        Err(e) => {
            tracing::warn!("DS failed to process message: {:?}", e);
            HttpResponse::InternalServerError().body(e.to_string())
        }
    }
}
