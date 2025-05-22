// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxprotos::{
    auth_service::v1::{
        ConnectRequest, ConnectResponse, EncryptedConnectionEstablishmentPackage,
        EnqueuePackageResponse, FetchConnectionPackageResponse, connect_request, connect_response,
    },
    validation::MissingFieldExt,
};
use phnxtypes::{identifiers::UserHandleHash, messages::client_as::ConnectionPackage};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tonic::{Status, Streaming};
use tracing::error;

use crate::auth_service::AuthService;

impl AuthService {
    /// Implements the Connect Handle protocol
    ///
    /// Meant to be executed in a background tokio task
    #[expect(
        clippy::wrong_self_convention,
        reason = "as actually means 'authentication service'"
    )]
    pub(crate) async fn as_connect_handle_protocol(
        self,
        incoming: Streaming<ConnectRequest>,
        outgoing: mpsc::Sender<Result<ConnectResponse, Status>>,
    ) {
        if let Err(error) = self
            .as_connect_handle_protocol_impl(incoming, &outgoing)
            .await
        {
            error!(%error, "error in connect handle protocol");
            let _ignore_closed_channel = outgoing.send(Err(error)).await;
        }
    }

    async fn as_connect_handle_protocol_impl(
        &self,
        mut incoming: Streaming<ConnectRequest>,
        outgoing: &mpsc::Sender<Result<ConnectResponse, Status>>,
    ) -> Result<(), Status> {
        // step 1: fetch connetion package for a handle hash
        let step = incoming.next().await;
        let fetch_connection_package = match step {
            Some(Ok(ConnectRequest {
                step: Some(connect_request::Step::Fetch(fetch)),
            })) => fetch,
            Some(Ok(_)) => {
                return Err(Status::failed_precondition(
                    "protocol violation: expected fetch",
                ));
            }
            Some(Err(error)) => {
                error!(%error, "error in connect handle protocol");
                return Ok(());
            }
            None => return Ok(()),
        };

        let hash = fetch_connection_package
            .hash
            .ok_or_missing_field("hash")?
            .try_into()?;

        let connection_package = self.as_get_connection_package_for_handle(hash).await?;
        if outgoing
            .send(Ok(ConnectResponse {
                step: Some(connect_response::Step::FetchResponse(
                    FetchConnectionPackageResponse {
                        connection_package: Some(connection_package.into()),
                    },
                )),
            }))
            .await
            .is_err()
        {
            return Ok(()); // protocol aborted
        }

        // step 2: enqueue encrypted connection establishment package
        let step = incoming.next().await;
        let enqueue_package = match step {
            Some(Ok(ConnectRequest {
                step: Some(connect_request::Step::Enqueue(enqueue_package)),
            })) => enqueue_package,
            Some(Ok(_)) => {
                return Err(Status::failed_precondition(
                    "protocol violation: expected enqueue",
                ));
            }
            Some(Err(error)) => {
                error!(%error, "error in connect handle protocol");
                return Ok(());
            }
            None => return Ok(()),
        };

        let connection_establishment_package = enqueue_package
            .connection_establishment_package
            .ok_or_missing_field("connection_establishment_package")?;

        self.as_enqueue_connection_package(connection_establishment_package)
            .await?;

        // acknowledge
        if outgoing
            .send(Ok(ConnectResponse {
                step: Some(connect_response::Step::EnqueueResponse(
                    EnqueuePackageResponse {},
                )),
            }))
            .await
            .is_err()
        {
            return Ok(()); // protocol aborted
        }

        Ok(())
    }

    async fn as_get_connection_package_for_handle(
        &self,
        _hash: UserHandleHash,
    ) -> Result<ConnectionPackage, GetConnectionPackageForHandleError> {
        todo!()
    }

    async fn as_enqueue_connection_package(
        &self,
        _connection_establishment_package: EncryptedConnectionEstablishmentPackage,
    ) -> Result<(), EnqueueConnectionPackageError> {
        todo!()
    }
}

#[derive(Debug)]
enum GetConnectionPackageForHandleError {}

impl From<GetConnectionPackageForHandleError> for Status {
    fn from(error: GetConnectionPackageForHandleError) -> Self {
        match error {}
    }
}

#[derive(Debug)]
enum EnqueueConnectionPackageError {}

impl From<EnqueueConnectionPackageError> for Status {
    fn from(error: EnqueueConnectionPackageError) -> Self {
        match error {}
    }
}
