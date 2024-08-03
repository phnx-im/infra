// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use actix_web::{
    web::{self, Data},
    HttpResponse, Responder,
};
use phnxbackend::{
    messages::qs_qs::QsToQsMessage,
    qs::{
        errors::QsEnqueueError, network_provider_trait::NetworkProvider,
        storage_provider_trait::QsStorageProvider, Qs, QsConnector,
    },
};
use phnxtypes::{
    errors::qs::QsVerifyingKeyError, messages::client_qs::VerifiableClientToQsMessage,
};
use tls_codec::{DeserializeBytes, Serialize};

pub mod push_notification_provider;
pub mod ws;

#[tracing::instrument(name = "Process QS message", skip_all)]
pub(crate) async fn qs_process_message<Qsp: QsStorageProvider>(
    qs_storage_provider: Data<Arc<Qsp>>,
    message: web::Bytes,
) -> impl Responder {
    // Extract the storage provider.
    let storage_provider = qs_storage_provider.get_ref();

    // Deserialize the message.
    let message = match VerifiableClientToQsMessage::tls_deserialize_exact_bytes(message.as_ref()) {
        Ok(message) => message,
        Err(e) => {
            tracing::warn!("QS received invalid message: {:?}", e);
            return HttpResponse::BadRequest().body(e.to_string());
        }
    };

    // Process the message.
    match Qs::process(storage_provider.as_ref(), message).await {
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

#[tracing::instrument(name = "Process federated QS message", skip_all)]
pub(crate) async fn qs_process_federated_message<
    Qc: QsConnector<EnqueueError = QsEnqueueError<S, N>, VerifyingKeyError = QsVerifyingKeyError>,
    S: QsStorageProvider,
    N: NetworkProvider,
>(
    qs_connector: Data<Qc>,
    storage_provider: Data<Arc<S>>,
    message: web::Bytes,
) -> impl Responder {
    // Deserialize the message.
    let message = match QsToQsMessage::tls_deserialize_exact_bytes(message.as_ref()) {
        Ok(message) => message,
        Err(e) => {
            tracing::warn!("QS received invalid federated message: {:?}", e);
            return HttpResponse::BadRequest().body(e.to_string());
        }
    };

    // Process the message.
    match Qs::process_federated_message(
        qs_connector.get_ref(),
        storage_provider.get_ref().as_ref(),
        message,
    )
    .await
    {
        // If the message was processed successfully, return the response.
        Ok(response) => {
            tracing::trace!("Processed federated message successfully");
            HttpResponse::Ok().body(response.tls_serialize_detached().unwrap())
        }
        // If the message could not be processed, return an error.
        Err(e) => {
            tracing::warn!("QS failed to process federated message: {:?}", e);
            HttpResponse::InternalServerError().body(e.to_string())
        }
    }
}
