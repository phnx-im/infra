// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(unused_variables)]

use mls_assist::{messages::AssistedWelcome, KeyPackage};
use opaque_ke::RegistrationRequest;
use tls_codec::{TlsDeserialize, TlsSerialize, TlsSize};

use crate::{
    crypto::{OpaqueCipherSuite, QueueRatchet, RatchetPublicKey},
    ds::group_state::TimeStamp,
};

use self::{
    devices::{AddDeviceError, GetDevicesError, RemoveDeviceError},
    invitations::InviteUserError,
    key_packages::{FetchKeyPackagesError, PublisKeyPackagesError},
    registration::{RegistrationError, RegistrationResponse},
    username::Username,
};

pub mod client_api;
pub mod codec;
pub mod credentials;
pub mod devices;
pub mod errors;
pub mod invitations;
pub mod key_packages;
pub mod registration;
pub mod storage_provider_trait;
pub mod username;

/*
Actions:
ACTION_AS_INITIATE_2FA_AUTHENTICATION

User:
ACTION_AS_INIT_USER_REGISTRATION
ACTION_AS_FINISH_USER_REGISTRATION
ACTION_AS_DELETE_USER

Client:
ACTION_AS_INITIATE_CLIENT_ADDITION
ACTION_AS_FINISH_CLIENT_ADDITION
ACTION_AS_DELETE_CLIENT
ACTION_AS_DEQUEUE_MESSAGES
ACTION_AS_PUBLISH_KEY_PACKAGES
ACTION_AS_CLIENT_KEY_PACKAGE

Anonymous:
ACTION_AS_USER_CLIENTS
ACTION_AS_USER_KEY_PACKAGES
ACTION_AS_ENQUEUE_MESSAGE
ACTION_AS_CREDENTIALS
*/

// === Authentication ===

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct OpaqueKe1 {}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct OpaqueKe2 {}

#[derive(Clone, Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct OpaqueKe3 {}

/// Registration request containing the OPAQUE payload.
///
/// The TLS serialization implementation of this
#[derive(Debug)]
pub(crate) struct OpaqueRegistrationRequest {
    client_message: RegistrationRequest<OpaqueCipherSuite>,
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub(crate) struct OpaqueRegistrationResponse {}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub(crate) struct OpaqueRegistrationRecord {}

// === User ===

pub struct AsUserId {
    pub client_id: Vec<u8>,
}

pub struct AsUserRecord {}

#[derive(Clone, Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct UserName {}

// === Client ===

#[derive(Clone, Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct AsClientId {
    pub(crate) client_id: Vec<u8>,
}

impl AsClientId {
    pub fn username(&self) -> UserName {
        todo!()
    }
}

pub struct AsClientRecord {
    pub queue_encryption_key: RatchetPublicKey,
    pub ratchet_key: QueueRatchet,
    pub activity_time: TimeStamp,
}

impl AsClientRecord {}

// === Legacy ===

pub struct AuthService {}

// === Authenticated endpoints ===
// TODO: Implement authentication

impl AuthService {
    /// Register a new user account.
    pub fn register_user(username: Username) -> Result<RegistrationResponse, RegistrationError> {
        todo!()
    }

    /// Add a device to a user account.
    pub fn add_device(username: Username, device: DeviceCertificate) -> Result<(), AddDeviceError> {
        todo!()
    }

    /// Remove a device from a user account.
    pub fn remove_device(
        username: Username,
        device: DeviceCertificate,
    ) -> Result<(), RemoveDeviceError> {
        todo!()
    }

    /// Get the list of devices for a user account.
    pub fn get_devices(username: Username) -> Result<Vec<DeviceCertificate>, GetDevicesError> {
        todo!()
    }

    /// Publish own KeyPackages.
    pub fn publish_key_packages(
        username: Username,
        key_packages: Vec<KeyPackage>,
    ) -> Result<(), PublisKeyPackagesError> {
        todo!()
    }
}

// === Pseudonymous endpoints ===

impl AuthService {
    /// Fetch KeyPackages from other users.
    pub fn fetch_key_packages(
        username: Username,
    ) -> Result<Vec<KeyPackage>, FetchKeyPackagesError> {
        todo!()
    }

    /// Invite another user to a group.
    pub fn invite_user(
        username: Username,
        welcome: AssistedWelcome,
    ) -> Result<(), InviteUserError> {
        todo!()
    }
}

// === Temporary data types ===
// TODO: This should be replaced with proper types once they become avaiable.

/// A certificate representing a user's device
pub struct DeviceCertificate {}
