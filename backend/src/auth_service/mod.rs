// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module contains the APIs of the authentication service (AS). It only
//! performs a limited amount of rate-limiting, so it should only be deployed
//! behind a rate-limiting module.
//!
//! NOTE: This document and the API stubs in this module represent a work in
//! progress and will likely change in their details. However, barring the
//! discovery of a major flaw in the current design, the general design of the
//! AS should remain the same.
//!
//! # Overview
//!
//! The AS main purpose is to act as certificate authority for clients of the
//! homeserver. More specifically, clients can request that the AS sign their
//! certificates and store some of their (public) authentication key material.
//!
//! For clients to verify that other clients belong to their claimed homeserver,
//! the AS also publishes its public key material.
//!
//! In addition, the AS allows clients to publish and continuously update their
//! public Evolving Identity state.
//!
//! # Certificate signing
//!
//! The AS acts as a certificate authority and allows clients to request signing
//! their certificates via the ACME protocol.
//!
//! This certificate is the client's main credential. In additions client may
//! use group-specific credentials that are in turn signed by the main
//! credential.
//!
//! TODO: Note, that this is not vanilla ACME, but a modified version with a
//! different verification procedure and one that signs certificate signing
//! requests for intermediate certificates. The last point is required for
//! clients to sign group-specific certificates such as the "missing link
//! certificates" (see definition of the DS).
//!
//! # Evolving Identity
//!
//! Users of a homeserver maintain an Evolving Identity (EID) state and
//! publish this state through the AS. This state consists of the public tree of
//! an MLS group that contains one member per client that the user has.
//!
//! The leaf of each client contains the client's main credential (not a
//! group-specific one).
//!
//! If the user adds or removes clients, the corresponding commit is sent to the
//! AS as an MLSPlaintext message, allowing the AS to keep track of changes to
//! the group. The commit is also broadcast to all groups the client is in
//! encapsuled in an (encrypted) MLS application message.

#![allow(unused_variables)]

use mls_assist::{messages::AssistedWelcome, KeyPackage};

use self::{
    devices::{AddDeviceError, GetDevicesError, RemoveDeviceError},
    invitations::InviteUserError,
    key_packages::{FetchKeyPackagesError, PublisKeyPackagesError},
    registration::{RegistrationError, RegistrationResponse},
    username::Username,
};

pub mod credentials;
pub mod devices;
pub mod invitations;
pub mod key_packages;
pub mod registration;
pub mod username;

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
