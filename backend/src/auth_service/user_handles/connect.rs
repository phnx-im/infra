// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use displaydoc::Display;
use futures_util::Stream;
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
use sqlx::PgPool;
use thiserror::Error;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tonic::{Status, Streaming};
use tracing::{debug, error};

use crate::auth_service::{AuthService, connection_package::StorableConnectionPackage};

use super::UserHandleRecord;

/// The protocol for a user connecting to another user via their handle
#[cfg_attr(test, mockall::automock)]
pub(crate) trait ConnectHandleProtocol {
    /// Implements the Connect Handle protocol
    async fn connect_handle_protocol(
        self,
        incoming: Streaming<ConnectRequest>,
        outgoing: mpsc::Sender<Result<ConnectResponse, Status>>,
    ) where
        Self: Sized,
    {
        run_protocol(&self, incoming, &outgoing).await
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

async fn run_protocol(
    protocol: &impl ConnectHandleProtocol,
    incoming: impl Stream<Item = Result<ConnectRequest, Status>> + Unpin,
    outgoing: &mpsc::Sender<Result<ConnectResponse, Status>>,
) {
    if let Err(error) = run_protocol_impl(protocol, incoming, outgoing).await {
        error!(%error, "error in connect handle protocol");
        let _ignore_closed_channel = outgoing.send(Err(error.into())).await;
    }
}

async fn run_protocol_impl(
    protocol: &impl ConnectHandleProtocol,
    mut incoming: impl Stream<Item = Result<ConnectRequest, Status>> + Unpin,
    outgoing: &mpsc::Sender<Result<ConnectResponse, Status>>,
) -> Result<(), ConnectProtocolError> {
    // step 1: fetch connetion package for a handle hash
    debug!("step 1: waiting for fetch connection package step");
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

    debug!("load user handle expiration data");
    let Some(expiration_data) = protocol.load_user_handle_expiration_data(&hash).await? else {
        return Err(ConnectProtocolError::HandleNotFound);
    };
    if !expiration_data.validate() {
        return Err(ConnectProtocolError::HandleNotFound);
    }

    debug!("get connection package for handle");
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
    debug!("step 2: waiting for enqueue package step");
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

    debug!("enqueue connection package");
    protocol
        .enqueue_connection_package(connection_establishment_package)
        .await?;

    // acknowledge
    debug!("acknowledge protocol finished");
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

    debug!("protocol finished");
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
        Self::load_user_handle_expiration_data_impl(&self.db_pool, hash).await
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

impl AuthService {
    async fn load_user_handle_expiration_data_impl(
        pool: &PgPool,
        hash: &UserHandleHash,
    ) -> sqlx::Result<Option<ExpirationData>> {
        let expiration_data = UserHandleRecord::load_expiration_data(pool, hash).await?;
        let Some(expiration_data) = expiration_data else {
            return Ok(None);
        };

        // Delete the handle if the expiration date has passed
        if !expiration_data.validate() {
            UserHandleRecord::delete(pool, hash).await?;
            return Ok(None);
        }

        Ok(Some(expiration_data))
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

#[cfg(test)]
mod tests {
    use std::time;

    use mockall::predicate::*;
    use phnxcommon::{credentials::keys::HandleVerifyingKey, identifiers::UserId, time::Duration};
    use phnxprotos::auth_service::v1::{self, EnqueuePackageStep, FetchConnectionPackageStep};
    use tokio::{sync::mpsc, task::JoinHandle, time::timeout};
    use tokio_stream::wrappers::ReceiverStream;

    use crate::auth_service::{
        client_record::persistence::tests::random_client_record,
        connection_package::persistence::tests::random_connection_package,
    };

    use super::*;

    fn init_test_tracing() {
        let _ = tracing_subscriber::fmt::fmt()
            .with_test_writer()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();
    }

    const PROTOCOL_TIMEOUT: time::Duration = time::Duration::from_secs(1);

    #[expect(clippy::type_complexity, reason = "usage in tests is straightforward")]
    fn run_test_protocol(
        mock_protocol: MockConnectHandleProtocol,
    ) -> (
        mpsc::Sender<Result<ConnectRequest, Status>>,
        mpsc::Receiver<Result<ConnectResponse, Status>>,
        JoinHandle<()>,
    ) {
        let (requests_tx, requests_rx) = mpsc::channel(10);
        let (responses_tx, responses_rx) = mpsc::channel(10);

        // run the protocol
        let run_handle = tokio::spawn(async move {
            timeout(
                PROTOCOL_TIMEOUT,
                run_protocol(
                    &mock_protocol,
                    ReceiverStream::new(requests_rx),
                    &responses_tx,
                ),
            )
            .await
            .expect("protocol handler timed out")
        });

        (requests_tx, responses_rx, run_handle)
    }

    #[tokio::test]
    async fn connect_handle_protocol_success() -> anyhow::Result<()> {
        init_test_tracing();

        let user_id = UserId::random("example.com".parse()?);
        let client_credential = random_client_record(user_id)?.credential().clone();

        let hash = UserHandleHash::new([1; 32]);
        let expiration_data = ExpirationData::new(Duration::days(1));
        let connection_package = random_connection_package(client_credential);
        let connection_establishment_package = EncryptedConnectionEstablishmentPackage::default();

        let mut mock_protocol = MockConnectHandleProtocol::new();

        mock_protocol
            .expect_load_user_handle_expiration_data()
            .with(eq(hash))
            .returning(move |_| Ok(Some(expiration_data.clone())));

        let inner_connection_package = connection_package.clone();
        mock_protocol
            .expect_get_connection_package_for_handle()
            .with(eq(hash))
            .returning(move |_| Ok(inner_connection_package.clone()));

        mock_protocol
            .expect_enqueue_connection_package()
            .with(eq(connection_establishment_package.clone()))
            .returning(|_| Ok(()));

        let (requests, mut responses, run_handle) = run_test_protocol(mock_protocol);

        let request_fetch = ConnectRequest {
            step: Some(connect_request::Step::Fetch(FetchConnectionPackageStep {
                hash: Some(hash.into()),
            })),
        };

        // step 1
        requests.send(Ok(request_fetch)).await.unwrap();
        match responses.recv().await.unwrap() {
            Ok(ConnectResponse {
                step:
                    Some(connect_response::Step::FetchResponse(FetchConnectionPackageResponse {
                        connection_package: Some(received_connection_package),
                    })),
            }) => {
                let connection_package_proto: v1::ConnectionPackage = connection_package.into();
                assert_eq!(connection_package_proto, received_connection_package);
            }
            _ => panic!("unexpected response type"),
        }

        // step 2
        let request_enqueue = ConnectRequest {
            step: Some(connect_request::Step::Enqueue(EnqueuePackageStep {
                connection_establishment_package: Some(connection_establishment_package.clone()),
            })),
        };
        requests.send(Ok(request_enqueue)).await.unwrap();
        match responses.recv().await.unwrap() {
            Ok(ConnectResponse {
                step: Some(connect_response::Step::EnqueueResponse(EnqueuePackageResponse {})),
            }) => {}
            _ => panic!("unexpected response type"),
        }

        run_handle.await.expect("protocol panicked");

        Ok(())
    }

    #[tokio::test]
    async fn connect_handle_protocol_handle_not_found() -> anyhow::Result<()> {
        init_test_tracing();

        let hash = UserHandleHash::new([1; 32]);

        let mut mock_protocol = MockConnectHandleProtocol::new();

        mock_protocol
            .expect_load_user_handle_expiration_data()
            .with(eq(hash))
            .returning(|_| Ok(None));

        let (requests, mut responses, run_handle) = run_test_protocol(mock_protocol);

        let request_fetch = ConnectRequest {
            step: Some(connect_request::Step::Fetch(FetchConnectionPackageStep {
                hash: Some(hash.into()),
            })),
        };

        requests.send(Ok(request_fetch)).await.unwrap();

        let response = responses.recv().await.unwrap();
        assert_eq!(response.unwrap_err().code(), tonic::Code::NotFound);

        run_handle.await.expect("protocol panicked");

        Ok(())
    }

    #[tokio::test]
    async fn connect_handle_protocol_handle_expired() -> anyhow::Result<()> {
        init_test_tracing();

        let hash = UserHandleHash::new([1; 32]);

        let mut mock_protocol = MockConnectHandleProtocol::new();

        mock_protocol
            .expect_load_user_handle_expiration_data()
            .with(eq(hash))
            .returning(|_| Ok(Some(ExpirationData::new(Duration::milliseconds(1)))));

        let (requests, mut responses, run_handle) = run_test_protocol(mock_protocol);

        let request_fetch = ConnectRequest {
            step: Some(connect_request::Step::Fetch(FetchConnectionPackageStep {
                hash: Some(hash.into()),
            })),
        };

        requests.send(Ok(request_fetch)).await.unwrap();

        let response = responses.recv().await.unwrap();
        assert_eq!(response.unwrap_err().code(), tonic::Code::NotFound);

        run_handle.await.expect("protocol panicked");

        Ok(())
    }

    #[tokio::test]
    async fn connect_handle_protocol_protocol_violation() -> anyhow::Result<()> {
        init_test_tracing();

        let mock_protocol = MockConnectHandleProtocol::new();
        let (requests, mut responses, run_handle) = run_test_protocol(mock_protocol);

        // empty requests in step 1

        requests
            .send(Ok(ConnectRequest { step: None }))
            .await
            .unwrap();
        let response = responses.recv().await.unwrap();
        assert_eq!(
            response.unwrap_err().code(),
            tonic::Code::FailedPrecondition
        );

        run_handle.await.expect("protocol panicked");

        let mock_protocol = MockConnectHandleProtocol::new();
        let (requests, mut responses, run_handle) = run_test_protocol(mock_protocol);

        // enqueue in step 1

        requests
            .send(Ok(ConnectRequest {
                step: Some(connect_request::Step::Enqueue(EnqueuePackageStep {
                    connection_establishment_package: None,
                })),
            }))
            .await
            .unwrap();
        let response = responses.recv().await.unwrap();
        assert_eq!(
            response.unwrap_err().code(),
            tonic::Code::FailedPrecondition
        );

        run_handle.await.expect("protocol panicked");

        // fetch in step 2

        let user_id = UserId::random("example.com".parse()?);
        let client_credential = random_client_record(user_id)?.credential().clone();

        let hash = UserHandleHash::new([1; 32]);
        let expiration_data = ExpirationData::new(Duration::days(1));
        let connection_package = random_connection_package(client_credential);

        let mut mock_protocol = MockConnectHandleProtocol::new();

        mock_protocol
            .expect_load_user_handle_expiration_data()
            .with(eq(hash))
            .returning(move |_| Ok(Some(expiration_data.clone())));

        let inner_connection_package = connection_package.clone();
        mock_protocol
            .expect_get_connection_package_for_handle()
            .with(eq(hash))
            .returning(move |_| Ok(inner_connection_package.clone()));

        let (requests, mut responses, run_handle) = run_test_protocol(mock_protocol);

        requests
            .send(Ok(ConnectRequest {
                step: Some(connect_request::Step::Fetch(FetchConnectionPackageStep {
                    hash: Some(hash.into()),
                })),
            }))
            .await
            .unwrap();
        let response = responses.recv().await.unwrap();
        assert!(response.is_ok());

        requests
            .send(Ok(ConnectRequest {
                step: Some(connect_request::Step::Fetch(FetchConnectionPackageStep {
                    hash: Some(hash.into()),
                })),
            }))
            .await
            .unwrap();
        let response = responses.recv().await.unwrap();
        assert_eq!(
            response.unwrap_err().code(),
            tonic::Code::FailedPrecondition
        );

        run_handle.await.expect("protocol panicked");

        Ok(())
    }

    #[sqlx::test]
    async fn load_user_handle_expiration_data_loads(pool: PgPool) -> anyhow::Result<()> {
        let hash = UserHandleHash::new([1; 32]);
        let expiration_data = ExpirationData::new(Duration::days(1));

        let record = UserHandleRecord {
            user_handle_hash: hash,
            verifying_key: HandleVerifyingKey::from_bytes(vec![1, 2, 3, 4, 5]),
            expiration_data: expiration_data.clone(),
        };
        record.store(&pool).await?;

        let expiration_data =
            AuthService::load_user_handle_expiration_data_impl(&pool, &hash).await?;
        assert_eq!(expiration_data.as_ref(), Some(&record.expiration_data));

        Ok(())
    }

    #[sqlx::test]
    async fn load_user_handle_expiration_data_deletes_expired_handle(
        pool: PgPool,
    ) -> anyhow::Result<()> {
        let hash = UserHandleHash::new([1; 32]);
        let expiration_data = ExpirationData::new(Duration::zero());

        let record = UserHandleRecord {
            user_handle_hash: hash,
            verifying_key: HandleVerifyingKey::from_bytes(vec![1, 2, 3, 4, 5]),
            expiration_data: expiration_data.clone(),
        };
        record.store(&pool).await?;

        UserHandleRecord::load_verifying_key(&pool, &hash)
            .await?
            .expect("handle should exist");

        let expiration_data =
            AuthService::load_user_handle_expiration_data_impl(&pool, &hash).await?;
        assert_eq!(expiration_data, None);

        // Check that the record is deleted
        let loaded = UserHandleRecord::load_verifying_key(&pool, &hash).await?;
        assert_eq!(loaded, None);

        Ok(())
    }
}
