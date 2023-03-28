use thiserror::Error;

/// Error fetching a message from the QS.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum AsDequeueError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
    /// Couldn't find the requested queue.
    #[error("Couldn't find the requested queue")]
    QueueNotFound,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum InitUserRegistrationError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
    /// User already exists
    #[error("User already exists")]
    UserAlreadyExists,
    /// Invalid CSR
    #[error("Invalid CSR")]
    InvalidCsr,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum FinishUserRegistrationError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
    /// Client credential not found
    #[error("Client credential not found")]
    ClientCredentialNotFound,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum DeleteUserError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum UserClientsError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum InitClientAdditionError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
    /// Client already exists
    #[error("Client already exists")]
    ClientAlreadyExists,
    /// Invalid CSR
    #[error("Invalid CSR")]
    InvalidCsr,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum FinishClientAdditionError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
    /// Client credential not found
    #[error("Client credential not found")]
    ClientCredentialNotFound,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum DeleteClientError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum PublishKeyPackageError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum ClientKeyPackageError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum UserKeyPackagesError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum EnqueueMessageError {
    /// Library error
    #[error("Library error")]
    LibraryError,
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
    /// Client not found
    #[error("Client not found")]
    ClientNotFound,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum IssueTokensError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
    /// Too many tokens
    #[error("Too many tokens")]
    TooManyTokens,
}
