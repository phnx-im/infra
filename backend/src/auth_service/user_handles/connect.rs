// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use displaydoc::Display;
use phnxcommon::{
    identifiers::UserHandleHash, messages::client_as::ConnectionPackage, time::ExpirationData,
};
use phnxprotos::{
    auth_service::{
        convert::UserHandleHashError,
        v1::{
            ConnectRequest, ConnectResponse, EncryptedConnectionEstablishmentPackage,
            EnqueuePackageResponse, FetchConnectionPackageResponse, connect_request,
            connect_response,
        },
    },
    validation::{MissingFieldError, MissingFieldExt},
};
use thiserror::Error;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tonic::{Status, Streaming};
use tracing::error;

use crate::auth_service::{AuthService, connection_package::StorableConnectionPackage};

use super::UserHandleRecord;

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
            let _ignore_closed_channel = outgoing.send(Err(error.into())).await;
        }
    }

    async fn load_user_handle_expiration_data(
        &self,
        hash: &UserHandleHash,
    ) -> sqlx::Result<Option<ExpirationData>>;

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
) -> Result<(), ConnectProtocolError> {
    // step 1: fetch connetion package for a handle hash
    let step = incoming.next().await;
    let fetch_connection_package = match step {
        Some(Ok(ConnectRequest {
            step: Some(connect_request::Step::Fetch(fetch)),
        })) => fetch,
        Some(Ok(_)) => {
            return Err(ConnectProtocolError::ProtocolViolation("expected fetch"));
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

    let Some(expiration_data) = protocol.load_user_handle_expiration_data(&hash).await? else {
        return Err(ConnectProtocolError::HandleNotFound);
    };
    if !expiration_data.validate() {
        return Err(ConnectProtocolError::HandleNotFound);
    }

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
            return Err(ConnectProtocolError::ProtocolViolation("expected enqueue"));
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

#[derive(Debug, Error, Display)]
pub(crate) enum ConnectProtocolError {
    /// Protocol violation: $0
    ProtocolViolation(&'static str),
    /// Database provider error
    Database(#[from] sqlx::Error),
    /// Handle not found
    HandleNotFound,
    /// Invalid hash: $0
    InvalidHash(#[from] UserHandleHashError),
    /// Missing required field in request
    MissingField(#[from] MissingFieldError<&'static str>),
    /// Failed to enqueue connection package
    Enqueue(#[from] EnqueueConnectionPackageError),
    /// Failed to get connection package for handle
    ConnectionPackage(#[from] GetConnectionPackageForHandleError),
}

impl From<ConnectProtocolError> for Status {
    fn from(error: ConnectProtocolError) -> Self {
        let msg = error.to_string();
        match error {
            ConnectProtocolError::ProtocolViolation(_) => Status::failed_precondition(msg),
            ConnectProtocolError::Database(error) => {
                error!(%error, "database error");
                Status::internal(msg)
            }
            ConnectProtocolError::HandleNotFound => Status::not_found(msg),
            ConnectProtocolError::Enqueue(_) => Status::internal(msg),
            ConnectProtocolError::MissingField(_) | ConnectProtocolError::InvalidHash(_) => {
                Status::invalid_argument(msg)
            }
            ConnectProtocolError::ConnectionPackage(error) => {
                error!(%error, "failed to get connection package for handle");
                Status::internal(msg)
            }
        }
    }
}

impl ConnectHandleProtocol for AuthService {
    async fn load_user_handle_expiration_data(
        &self,
        hash: &UserHandleHash,
    ) -> sqlx::Result<Option<ExpirationData>> {
        UserHandleRecord::load_expiration_data(&self.db_pool, *hash).await
    }

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

#[derive(Debug, Error, Clone)]
pub(crate) enum EnqueueConnectionPackageError {}

impl From<EnqueueConnectionPackageError> for Status {
    fn from(error: EnqueueConnectionPackageError) -> Self {
        match error {}
    }
}
