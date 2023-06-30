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
use tls_codec::{DeserializeBytes, Serialize};

/// DS endpoint for all group-based functionalities.
#[utoipa::path(
    post,
    path = "{ENDPOINT_DS_GROUPS}",
    tag = "DS GROUPS",
    request_body = VerifiableClientToDsMessage,
    responses(
        (status = 200, description = "Message processed successfully."),
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
    let message = match VerifiableClientToDsMessage::tls_deserialize_exact(&message) {
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
            let serialized_response = response.tls_serialize_detached().unwrap();
            HttpResponse::Ok().body(serialized_response)
        }
        // If the message could not be processed, return an error.
        Err(e) => {
            tracing::warn!("DS failed to process message: {:?}", e);
            HttpResponse::InternalServerError().body(e.to_string())
        }
    }
}

/// DS endpoint to fetch group ids.
#[utoipa::path(
    post,
    path = "{ENDPOINT_DS_GROUP_IDS}",
    tag = "DS GROUP IDS",
    responses(
        (status = 200, description = "Issued group ID."),
    )
)]
#[tracing::instrument(name = "Issue group id", skip_all)]
pub(crate) async fn ds_request_group_id<Dsp: DsStorageProvider>(
    ds_storage_provider: Data<Dsp>,
) -> impl Responder {
    // Extract the storage provider.
    let storage_provider = ds_storage_provider.get_ref();
    // Create a new group on the DS.
    let group_id = DsApi::request_group_id(storage_provider).await;

    HttpResponse::Ok().body(group_id.tls_serialize_detached().unwrap())
}
