// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! PHNX server.

pub mod configurations;
pub mod endpoints;
pub mod storage_provider;
pub mod telemetry;

use endpoints::{ds::*, *};

use actix_web::{
    dev::Server,
    web::{self, Data},
    App, HttpServer,
};
use phnxbackend::{
    auth_service::storage_provider_trait::{AsEphemeralStorageProvider, AsStorageProvider},
    ds::DsStorageProvider,
    qs::{storage_provider_trait::QsStorageProvider, QsConnector},
};
use std::{net::TcpListener, sync::Arc};
use tracing_actix_web::TracingLogger;

use crate::endpoints::{
    auth_service::as_process_message,
    health_check,
    qs::{
        qs_process_message,
        ws::{upgrade_connection, DispatchWebsocketNotifier},
    },
};

/// Configure and run the server application.
pub fn run<
    Dsp: DsStorageProvider,
    Qsp: QsStorageProvider,
    Qep: QsConnector,
    Asp: AsStorageProvider,
    Aesp: AsEphemeralStorageProvider,
>(
    listener: TcpListener,
    ws_dispatch_notifier: DispatchWebsocketNotifier,
    ds_storage_provider: Dsp,
    qs_storage_provider: Arc<Qsp>,
    as_storage_provider: Asp,
    as_ephemeral_storage_provider: Aesp,
    qs_connector: Qep,
) -> Result<Server, std::io::Error> {
    // Wrap providers in a Data<T>
    let ds_storage_provider_data = Data::new(ds_storage_provider);
    let qs_storage_provider_data = Data::new(qs_storage_provider);
    let as_storage_provider_data = Data::new(as_storage_provider);
    let as_ephemeral_storage_provider_data = Data::new(as_ephemeral_storage_provider);
    let qs_connector_data = Data::new(qs_connector);
    let ws_dispatch_notifier_data = Data::new(ws_dispatch_notifier.dispatch_addr);

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
        let app = App::new()
            .wrap(TracingLogger::default())
            .route(ENDPOINT_HEALTH_CHECK, web::get().to(health_check))
            .app_data(ds_storage_provider_data.clone())
            .app_data(qs_storage_provider_data.clone())
            .app_data(as_storage_provider_data.clone())
            .app_data(as_ephemeral_storage_provider_data.clone())
            .app_data(qs_connector_data.clone())
            // DS enpoint
            .route(
                ENDPOINT_DS_GROUPS,
                web::post().to(ds_process_message::<Dsp, Qep>),
            )
            // QS endpoint
            .route(ENDPOINT_QS, web::post().to(qs_process_message::<Qsp>))
            // QS endpoint
            .route(ENDPOINT_AS, web::post().to(as_process_message::<Asp, Aesp>))
            // WS endpoint
            .app_data(ws_dispatch_notifier_data.clone())
            .route(ENDPOINT_QS_WS, web::get().to(upgrade_connection));
        app
    })
    .listen(listener)?
    .run();
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
