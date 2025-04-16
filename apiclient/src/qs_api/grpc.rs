// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxprotos::queue_service::v1::queue_service_client::QueueServiceClient;
use tonic::transport::Channel;

#[derive(Debug, Clone)]
pub(crate) struct QsGrpcClient {
    client: QueueServiceClient<Channel>,
}

impl QsGrpcClient {
    pub(crate) fn new(client: QueueServiceClient<Channel>) -> Self {
        Self { client }
    }
}
