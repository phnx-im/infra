use thiserror::*;

#[derive(Debug, Error)]
pub enum InviteUserError {
    #[error("User not found")]
    UserNotFound,
    #[error("Wrong devices")]
    WrongDevices,
}
