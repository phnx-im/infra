// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use actix_web::{HttpResponse, Responder};

pub mod auth_service;
pub(crate) mod ds;
pub mod qs;

/// DS endpoints
pub const ENDPOINT_DS_GROUPS: &str = "/ds_groups";

/// QS endpoints
pub const ENDPOINT_QS: &str = "/qs";
pub const ENDPOINT_QS_FEDERATION: &str = "/qs_federation";
pub const ENDPOINT_QS_WS: &str = "/qs/ws/"; // WebSocket endpoints must end with a slash.

/// AS endpoints
pub const ENDPOINT_AS: &str = "/as";

/// Health check endpoint
pub const ENDPOINT_HEALTH_CHECK: &str = "/health_check";

pub(crate) async fn health_check() -> impl Responder {
    HttpResponse::Ok()
}
