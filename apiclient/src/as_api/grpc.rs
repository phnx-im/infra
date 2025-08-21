// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use airprotos::auth_service::v1::auth_service_client::AuthServiceClient;
use tonic::transport::Channel;

#[derive(Debug, Clone)]
pub(crate) struct AsGrpcClient {
    client: AuthServiceClient<Channel>,
}

impl AsGrpcClient {
    pub(crate) fn new(client: AuthServiceClient<Channel>) -> Self {
        Self { client }
    }

    pub(super) fn client(&self) -> AuthServiceClient<Channel> {
        self.client.clone()
    }
}
