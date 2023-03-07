use thiserror::*;

use crate::qs::QsClientId;

pub struct RegistrationResponse {
    pub welcome_queue_id: QsClientId,
}

#[derive(Debug, Error)]
pub enum RegistrationError {
    #[error("Username is not valid")]
    UsernameInvalid,
    #[error("Username is already taken")]
    UsernameTaken,
    #[error("An internal server error occurred")]
    ServerError,
}
