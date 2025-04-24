// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Server that makes the logic implemented in the backend available to clients via a REST API

pub mod configurations;
mod connect_info;
pub mod endpoints;
pub mod enqueue_provider;
pub mod network_provider;
pub mod telemetry;

use connect_info::ConnectInfoInterceptor;
use endpoints::{ds::*, qs::ws::DispatchWebsocketNotifier};

use actix_web::{
    App, HttpServer,
    web::{self, Data},
};
use phnxbackend::{
    auth_service::AuthService,
    ds::{Ds, GrpcDs},
    qs::{Qs, QsConnector, errors::QsEnqueueError, network_provider::NetworkProvider},
};
use phnxprotos::delivery_service::v1::delivery_service_server::DeliveryServiceServer;
use phnxtypes::endpoint_paths::{
    ENDPOINT_AS, ENDPOINT_DS_GROUPS, ENDPOINT_HEALTH_CHECK, ENDPOINT_QS, ENDPOINT_QS_FEDERATION,
    ENDPOINT_QS_WS,
};
use std::{io, net::TcpListener, time::Duration};
use tokio_stream::wrappers::TcpListenerStream;
use tonic::service::InterceptorLayer;
use tower_governor::{
    GovernorLayer, governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor,
};
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::{Level, enabled, info};
use tracing_actix_web::TracingLogger;

use crate::endpoints::{
    auth_service::as_process_message,
    health_check,
    qs::{qs_process_federated_message, qs_process_message, ws::upgrade_connection},
};

pub struct ServerRunParams<Qc, Np> {
    pub listener: TcpListener,
    pub grpc_listener: tokio::net::TcpListener,
    pub ds: Ds,
    pub auth_service: AuthService,
    pub qs: Qs,
    pub qs_connector: Qc,
    pub network_provider: Np,
    pub ws_dispatch_notifier: DispatchWebsocketNotifier,
    pub rate_limits: RateLimitsConfig,
}

/// Every `period`, allow bursts of up to `burst_size`-many requests, and replenish one element
/// after the `period`.
pub struct RateLimitsConfig {
    pub period: Duration,
    pub burst_size: u32,
}

/// Configure and run the server application.
pub fn run<Qc: QsConnector<EnqueueError = QsEnqueueError<Np>> + Clone, Np: NetworkProvider>(
    ServerRunParams {
        listener,
        grpc_listener,
        ds,
        auth_service,
        qs,
        qs_connector,
        network_provider,
        ws_dispatch_notifier,
        rate_limits,
    }: ServerRunParams<Qc, Np>,
) -> Result<impl Future<Output = io::Result<()>>, io::Error> {
    // Wrap providers in a Data<T>
    let ds_data = Data::new(ds.clone());
    let auth_service_data = Data::new(auth_service);
    let qs_data = Data::new(qs);
    let qs_connector_data = Data::new(qs_connector.clone());
    let network_provider_data = Data::new(network_provider);
    let ws_dispatch_notifier_data = Data::new(ws_dispatch_notifier);

    let http_addr = listener.local_addr().expect("Could not get local address");
    let grpc_addr = grpc_listener
        .local_addr()
        .expect("Could not get local address");

    info!(%http_addr, %grpc_addr, "Starting server");

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
    .listen_auto_h2c(listener)?
    .run();

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    // GRPC server
    let grpc_ds = GrpcDs::new(ds, qs_connector);

    let RateLimitsConfig { period, burst_size } = rate_limits;
    let governor_config = GovernorConfigBuilder::default()
        .period(period)
        .burst_size(burst_size)
        .key_extractor(SmartIpKeyExtractor)
        .finish()
        .expect("invalid governor config");

    // task cleaning up limiter tokens
    let governor_limiter = governor_config.limiter().clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            governor_limiter.retain_recent();
        }
    });

    tokio::spawn(async move {
        tonic::transport::Server::builder()
            .layer(InterceptorLayer::new(ConnectInfoInterceptor))
            .layer(
                TraceLayer::new_for_grpc()
                    .make_span_with(
                        DefaultMakeSpan::new()
                            .level(Level::INFO)
                            .include_headers(enabled!(Level::DEBUG)),
                    )
                    .on_request(DefaultOnRequest::new().level(Level::INFO))
                    .on_response(
                        DefaultOnResponse::new()
                            .level(Level::INFO)
                            .include_headers(enabled!(Level::DEBUG)),
                    ),
            )
            .layer(GovernorLayer::new(governor_config))
            .add_service(DeliveryServiceServer::new(grpc_ds))
            .serve_with_incoming_shutdown(TcpListenerStream::new(grpc_listener), async move {
                shutdown_rx.await.ok();
            })
            .await
            .expect("grpc server failed");
    });

    Ok(async move {
        let res = server.await;
        shutdown_tx.send(()).ok();
        res
    })
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
