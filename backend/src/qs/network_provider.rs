// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt::Debug;

use super::{Fqdn, qs_api::FederatedProcessingResult};

#[expect(async_fn_in_trait)]
pub trait NetworkProvider: Sync + Send + Debug + 'static {
    type NetworkError: std::error::Error;

    async fn deliver(
        &self,
        bytes: Vec<u8>,
        destination: Fqdn,
    ) -> Result<FederatedProcessingResult, Self::NetworkError>;
}
