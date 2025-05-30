// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Implements the local and the federation part of the protocol logic on the server side

pub mod auth_service;
pub mod ds;
pub(crate) mod errors;
pub mod infra_service;
pub mod messages;
pub mod qs;
pub mod rate_limiter;
pub mod settings;

pub use mls_assist::messages::{AssistedGroupInfo, AssistedMessageOut};
