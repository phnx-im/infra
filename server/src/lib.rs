// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Server that makes the logic implemented in the backend available to clients via a REST API

pub mod configurations;
pub mod endpoints;
pub mod enqueue_provider;
pub mod network_provider;
pub mod telemetry;

use endpoints::{ds::*, qs::ws::DispatchWebsocketNotifier};

use actix_web::{
    App, HttpServer,
    dev::Server,
    web::{self, Data},
};
use phnxbackend::{
    auth_service::AuthService,
    ds::{Ds, GrpcDs},
    qs::{Qs, QsConnector, errors::QsEnqueueError, network_provider::NetworkProvider},
};
use phnxtypes::endpoint_paths::{
    ENDPOINT_AS, ENDPOINT_DS_GROUPS, ENDPOINT_HEALTH_CHECK, ENDPOINT_QS, ENDPOINT_QS_FEDERATION,
    ENDPOINT_QS_WS,
};
use protos::delivery_service::v1::delivery_service_server::DeliveryServiceServer;
use std::net::TcpListener;
use tower_http::trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;
use tracing_actix_web::TracingLogger;

use crate::endpoints::{
    auth_service::as_process_message,
    health_check,
    qs::{qs_process_federated_message, qs_process_message, ws::upgrade_connection},
};

/// Configure and run the server application.
#[allow(clippy::too_many_arguments)]
pub fn run<Qc: QsConnector<EnqueueError = QsEnqueueError<Np>> + Clone, Np: NetworkProvider>(
    listener: TcpListener,
    ds: Ds,
    auth_service: AuthService,
    qs: Qs,
    qs_connector: Qc,
    network_provider: Np,
    ws_dispatch_notifier: DispatchWebsocketNotifier,
) -> Result<Server, std::io::Error> {
    // Wrap providers in a Data<T>
    let ds_data = Data::new(ds.clone());
    let auth_service_data = Data::new(auth_service);
    let qs_data = Data::new(qs);
    let qs_connector_data = Data::new(qs_connector.clone());
    let network_provider_data = Data::new(network_provider);
    let ws_dispatch_notifier_data = Data::new(ws_dispatch_notifier);

    let local_addr = listener.local_addr().expect("Could not get local address");

    tracing::info!(
        "Starting server, listening on {}:{}",
        listener
            .local_addr()
            .expect("Could not get local address")
            .ip(),
        listener
            .local_addr()
            .expect("Could not get local address")
            .port()
    );

    // Create & run the server
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route(ENDPOINT_HEALTH_CHECK, web::get().to(health_check))
            .app_data(ds_data.clone())
            .app_data(auth_service_data.clone())
            .app_data(qs_data.clone())
            .app_data(qs_connector_data.clone())
            .app_data(network_provider_data.clone())
            .app_data(ws_dispatch_notifier_data.clone())
            // DS enpoint
            .route(ENDPOINT_DS_GROUPS, web::post().to(ds_process_message::<Qc>))
            // QS endpoint
            .route(ENDPOINT_QS, web::post().to(qs_process_message))
            // QS federationendpoint
            .route(
                ENDPOINT_QS_FEDERATION,
                web::post().to(qs_process_federated_message::<Qc, Np>),
            )
            // QS endpoint
            .route(ENDPOINT_AS, web::post().to(as_process_message))
            // WS endpoint
            .route(ENDPOINT_QS_WS, web::get().to(upgrade_connection))
    })
    .listen(listener)?
    .run();

    // GRPC server

    let ip_addr = local_addr.ip();
    const GRPC_PORT: u16 = 50051;
    tracing::info!(%ip_addr, port = GRPC_PORT, "Starting gRPC server",);

    let grpc_ds = GrpcDs::new(ds, qs_connector);

    tokio::spawn(async move {
        tonic::transport::Server::builder()
            .layer(
                TraceLayer::new_for_grpc()
                    .on_request(DefaultOnRequest::new().level(Level::INFO))
                    .on_response(DefaultOnResponse::new().level(Level::INFO)),
            )
            .add_service(DeliveryServiceServer::new(grpc_ds))
            .serve(
                format!("{ip_addr}:{GRPC_PORT}")
                    .parse()
                    .expect("invalid address"),
            )
            .await
            .expect("grpc server failed");
    });

    Ok(server)
}

// QS endpoints

// Create pseudonymous user record:
// Input:
// User auth key
// Friendship token
// Client record
// Owning client auth key
// Owner HPKE key (queue encryption)
// Output:
// User UUID
// Client UUID

// Get pseudonymous user record:
// Input:
// User auth key (used as a signing key)
// User UUID

// Modify pseudonymous user record:
// Input:
// User auth key (used as a signing key)
// New auser auth key
// Friendship token
