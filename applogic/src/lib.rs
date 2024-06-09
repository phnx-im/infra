// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//#[path = "frb_generated.rs"]
pub(crate) mod frb_generated;
pub(crate) use frb_generated::*;

pub mod api;
pub(crate) mod app_state;
pub(crate) mod notifications;
