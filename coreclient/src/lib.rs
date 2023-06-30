// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#[macro_use]
mod errors;
mod contacts;
mod conversations;
mod groups;
pub mod notifications;
mod providers;
pub mod types;
pub mod users;
mod utils;

#[cfg(feature = "dart-bridge")]
mod dart_api;

use std::collections::HashMap;

pub(crate) use crate::errors::*;
use crate::{conversations::*, groups::*, types::*};

use notifications::{Notifiable, NotificationHub};
pub(crate) use openmls::prelude::*;
pub(crate) use openmls_rust_crypto::OpenMlsRustCrypto;

use uuid::Uuid;
