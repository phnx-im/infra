// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use async_trait::async_trait;
use std::error::Error;
use std::fmt::Debug;
use std::net::SocketAddr;

use super::Fqdn;

#[async_trait]
pub trait DnsProvider: Sync + Send + 'static {
    type DnsError: Error + Debug + Clone;

    async fn resolve(&self, fqdn: &Fqdn) -> Result<SocketAddr, Self::DnsError>;
}
