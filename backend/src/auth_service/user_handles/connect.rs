// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use displaydoc::Display;
use phnxprotos::{
    auth_service::v1::{
        ConnectRequest, ConnectResponse, EncryptedConnectionEstablishmentPackage,
        EnqueuePackageResponse, FetchConnectionPackageResponse, connect_request, connect_response,
    },
    validation::MissingFieldExt,
};
use phnxtypes::{identifiers::UserHandleHash, messages::client_as::ConnectionPackage};
use thiserror::Error;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tonic::{Status, Streaming};
use tracing::error;

use crate::auth_service::{AuthService, connection_package::StorableConnectionPackage};

/// The protocol for a user connecting to another user via their handle
pub(crate) trait ConnectHandleProtocol {
    /// Implements the Connect Handle protocol
    async fn connect_handle_protocol(
        self,
        incoming: Streaming<ConnectRequest>,
        outgoing: mpsc::Sender<Result<ConnectResponse, Status>>,
    ) where
        Self: Sized,
    {
        if let Err(error) = protocol_impl(&self, incoming, &outgoing).await {
            error!(%error, "error in connect handle protocol");
            let _ignore_closed_channel = outgoing.send(Err(error)).await;
        }
    }

    async fn get_connection_package_for_handle(
        &self,
        hash: UserHandleHash,
    ) -> Result<ConnectionPackage, GetConnectionPackageForHandleError>;

    async fn enqueue_connection_package(
        &self,
        connection_establishment_package: EncryptedConnectionEstablishmentPackage,
    ) -> Result<(), EnqueueConnectionPackageError>;
}

async fn protocol_impl(
    protocol: &impl ConnectHandleProtocol,
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

    let connection_package = protocol.get_connection_package_for_handle(hash).await?;
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

    protocol
        .enqueue_connection_package(connection_establishment_package)
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

impl ConnectHandleProtocol for AuthService {
    async fn get_connection_package_for_handle(
        &self,
        hash: UserHandleHash,
    ) -> Result<ConnectionPackage, GetConnectionPackageForHandleError> {
        StorableConnectionPackage::load_for_handle(&self.db_pool, &hash)
            .await
            .map_err(From::from)
    }

    async fn enqueue_connection_package(
        &self,
        _connection_establishment_package: EncryptedConnectionEstablishmentPackage,
    ) -> Result<(), EnqueueConnectionPackageError> {
        todo!("missing implementation of handle queue")
    }
}

#[derive(Debug, Error, Display)]
pub(crate) enum GetConnectionPackageForHandleError {
    /// Storage provider error
    Storage(#[from] sqlx::Error),
}

impl From<GetConnectionPackageForHandleError> for Status {
    fn from(error: GetConnectionPackageForHandleError) -> Self {
        let msg = error.to_string();
        match error {
            GetConnectionPackageForHandleError::Storage(error) => {
                error!(%error, "failed to get a connection package for handle");
                Status::internal(msg)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum EnqueueConnectionPackageError {}

impl From<EnqueueConnectionPackageError> for Status {
    fn from(error: EnqueueConnectionPackageError) -> Self {
        match error {}
    }
}
