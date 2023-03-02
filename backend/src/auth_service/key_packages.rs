use thiserror::*;

#[derive(Debug, Error)]
pub enum PublisKeyPackagesError {
    #[error("User not found")]
    UserNotFound,
    #[error("Invalid KeyPackages")]
    InvalidKeyPackages,
}

#[derive(Debug, Error)]
pub enum FetchKeyPackagesError {
    #[error("User not found")]
    UserNotFound,
    #[error("No KeyPackages available")]
    NoKeyPackagesAvailable,
}
