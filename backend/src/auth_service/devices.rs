use thiserror::*;

#[derive(Debug, Error)]
pub enum AddDeviceError {
    #[error("User not found")]
    UserNotFound,
    #[error("Device already exists")]
    DeviceExists,
}

#[derive(Debug, Error)]
pub enum RemoveDeviceError {
    #[error("User not found")]
    UserNotFound,
    #[error("Device not found")]
    DeviceNotFound,
}

#[derive(Debug, Error)]
pub enum GetDevicesError {
    #[error("User not found")]
    UserNotFound,
}
