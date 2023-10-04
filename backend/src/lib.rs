// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//#![allow(dead_code)]
//#![allow(unused_variables)]
#![deny(unreachable_pub)]

pub mod auth_service;
pub mod ds;
pub mod messages;
pub mod qs;

pub use mls_assist::messages::{AssistedGroupInfo, AssistedMessageOut};
