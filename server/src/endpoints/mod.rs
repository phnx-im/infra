// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use actix_web::{HttpResponse, Responder};

pub mod auth_service;
pub(crate) mod ds;
pub mod qs;

pub(crate) async fn health_check() -> impl Responder {
    HttpResponse::Ok()
}
