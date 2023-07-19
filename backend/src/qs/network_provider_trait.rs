// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use async_trait::async_trait;
use std::error::Error;
use std::fmt::Debug;

use super::Fqdn;

#[async_trait]
pub trait NetworkProvider: Sync + Send + Debug + 'static {
    type NetworkError: Error + Debug + Clone;

    async fn deliver(&self, bytes: Vec<u8>, destination: Fqdn) -> Result<(), Self::NetworkError>;
}
