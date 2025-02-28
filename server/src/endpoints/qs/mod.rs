// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use actix_web::{
    web::{self, Data},
    HttpResponse, Responder,
};
use phnxbackend::{
    messages::qs_qs::QsToQsMessage,
    qs::{errors::QsEnqueueError, network_provider_trait::NetworkProvider, Qs, QsConnector},
};
use phnxtypes::{
    errors::qs::QsProcessError, messages::client_qs::VerifiableClientToQsMessage,
    ACCEPTED_API_VERSIONS_HEADER,
};
use tls_codec::{DeserializeBytes, Serialize};
use tracing::{error, info, trace};

pub mod push_notification_provider;
pub mod ws;

#[tracing::instrument(name = "Process QS message", skip_all)]
pub(crate) async fn qs_process_message(qs: Data<Qs>, message: web::Bytes) -> impl Responder {
    // Extract the storage provider.

    // Deserialize the message.
    let message = match VerifiableClientToQsMessage::tls_deserialize_exact_bytes(message.as_ref()) {
        Ok(message) => message,
        Err(error) => {
            error!(%error, "QS received invalid message");
            return HttpResponse::BadRequest().body(error.to_string());
        }
    };

    // Process the message.
    match qs.process(message).await {
        // If the message was processed successfully, return the response.
        Ok(response) => {
            trace!("Processed message successfully");
            HttpResponse::Ok().body(response.tls_serialize_detached().unwrap())
        }
        Err(QsProcessError::Api(version_error)) => {
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
            error!(%error, "QS failed to process message");
            HttpResponse::InternalServerError().body(error.to_string())
        }
    }
}

#[tracing::instrument(name = "Process federated QS message", skip_all)]
pub(crate) async fn qs_process_federated_message<
    Qc: QsConnector<EnqueueError = QsEnqueueError<N>>,
    N: NetworkProvider,
>(
    qs_connector: Data<Qc>,
    qs: Data<Qs>,
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
    match qs
        .process_federated_message(qs_connector.get_ref(), message)
        .await
    {
        // If the message was processed successfully, return the response.
        Ok(response) => {
            trace!("Processed federated message successfully");
            HttpResponse::Ok().body(response.tls_serialize_detached().unwrap())
        }
        // If the message could not be processed, return an error.
        Err(e) => {
            tracing::warn!("QS failed to process federated message: {:?}", e);
            HttpResponse::InternalServerError().body(e.to_string())
        }
    }
}
