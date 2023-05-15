// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! PHNX server.

pub mod configurations;
pub mod endpoints;
pub mod storage_provider;
pub mod telemetry;

use actix::{Actor, Addr};
use endpoints::{ds::*, *};

use actix_web::{
    dev::Server,
    web::{self, Data},
    App, HttpServer,
};
use phnxbackend::{
    ds::DsStorageProvider,
    qs::{storage_provider_trait::QsStorageProvider, Qs, QsEnqueueProvider},
};
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

use crate::endpoints::qs::qs_process_message;

/// Configure and run the server application.
pub fn run<Dsp: DsStorageProvider, Qsp: QsStorageProvider, Qep: QsEnqueueProvider + 'static>(
    listener: TcpListener,
    ds_storage_provider: Dsp,
    qs_storage_provider: Qsp,
    qs_enqueue_provider: Qep,
) -> Result<Server, std::io::Error> {
    // Wrap providers in a Data<T>
    let ds_storage_provider_data = Data::new(ds_storage_provider);
    let qs_storage_provider_data = Data::new(qs_storage_provider);
    let qs_enqueue_provider_data = Data::new(qs_enqueue_provider);

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
            .app_data(ds_storage_provider_data.clone())
            .app_data(qs_storage_provider_data.clone())
            .app_data(qs_enqueue_provider_data.clone())
            // DS enpoint
            .route(ENDPOINT_DS, web::post().to(ds_process_message::<Dsp, Qep>))
            // QS endpoint
            .route(ENDPOINT_QS, web::post().to(qs_process_message::<Qsp>));
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
