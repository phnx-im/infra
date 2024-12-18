// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Multi-platform client application logic

pub(crate) use frb_generated::*;

pub mod api;
pub mod background_execution;

pub(crate) mod app_state;
pub(crate) mod frb_generated;
pub(crate) mod logging;
pub(crate) mod notifier;
pub(crate) mod util;
