// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxprotos::queue_service::v1::{
    ListenRequest, ListenResponse, queue_service_client::QueueServiceClient,
};
use phnxtypes::identifiers::QsClientId;
use tokio_stream::{Stream, StreamExt};
use tonic::transport::Channel;
use tracing::error;

use super::QsRequestError;

#[derive(Debug, Clone)]
pub(crate) struct QsGrpcClient {
    client: QueueServiceClient<Channel>,
}

impl QsGrpcClient {
    pub(crate) fn new(client: QueueServiceClient<Channel>) -> Self {
        Self { client }
    }

    pub(crate) async fn listen(
        &self,
        queue_id: QsClientId,
    ) -> Result<impl Stream<Item = ListenResponse> + use<>, QsRequestError> {
        let request = ListenRequest {
            client_id: Some(queue_id.into()),
        };
        let response = self.client.clone().listen(request).await?;
        let stream = response.into_inner().map_while(|response| {
            response
                .inspect_err(|status| error!(?status, "terminating listen stream due to an error"))
                .ok()
        });
        Ok(stream)
    }
}
