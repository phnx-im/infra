// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{error::Error, fmt::Debug};

use async_trait::async_trait;
use phnxtypes::{
    crypto::hpke::ClientIdDecryptionKey,
    identifiers::QsUserId,
    keypackage_batch::QsEncryptedAddPackage,
    messages::{FriendshipToken, QueueMessage},
};

use super::{client_record::QsClientRecord, Fqdn, QsClientId};

/// Storage provider trait for the QS.
#[async_trait]
pub trait QsStorageProvider: Sync + Send + Debug + 'static {
    type EnqueueError: Error + Debug;
    type ReadAndDeleteError: Error + Debug;

    type StoreKeyPackagesError: Error + Debug;
    type LoadUserKeyPackagesError: Error + Debug;

    type LoadSigningKeyError: Error + Debug;
    type LoadDecryptionKeyError: Error + Debug;

    type LoadConfigError: Error + Debug;

    async fn own_domain(&self) -> Fqdn;

    // === USERS ===

    // === CLIENTS ===

    // === KEY PACKAGES ===

    // All key package endpoints (at least for now) need to preserve the
    // invariant that there should always be at least one key package. Fetching
    // a key package for an existing client should thus never fail.

    /// Store key packages for a specific client.
    async fn store_key_packages(
        &self,
        client_id: &QsClientId,
        encrypted_key_packages: Vec<QsEncryptedAddPackage>,
    ) -> Result<(), Self::StoreKeyPackagesError>;

    /// Store a last resort key package for a specific client.
    async fn store_last_resort_key_package(
        &self,
        client_id: &QsClientId,
        encrypted_key_package: QsEncryptedAddPackage,
    ) -> Result<(), Self::StoreKeyPackagesError>;

    /// Return a key package for a specific client. The user ID is used to check if
    /// the client belongs to the user.
    /// TODO: This should probably check for expired KeyPackages
    async fn load_key_package(
        &self,
        user_id: &QsUserId,
        client_id: &QsClientId,
    ) -> Option<QsEncryptedAddPackage>;

    /// Return a key package for each client of a user referenced by a
    /// friendship token.
    /// TODO: This should probably check for expired KeyPackages
    async fn load_user_key_packages(
        &self,
        friendship_token: &FriendshipToken,
    ) -> Result<Vec<QsEncryptedAddPackage>, Self::LoadUserKeyPackagesError>;

    // === MESSAGES ===

    /// Append the given message to the queue. Returns an error if the payload
    /// is greater than the maximum payload allowed by the storage provider.
    /// TODO: Currently, the encryption layer is in control of the sequence
    /// numbers. This function assumes that messages are always enqueued in
    /// ascending order.
    async fn enqueue(
        &self,
        client_id: &QsClientId,
        message: QueueMessage,
    ) -> Result<(), Self::EnqueueError>;

    /// Delete all messages older than the given sequence number in the queue
    /// with the given client ID and return up to the requested number of
    /// messages from the queue starting with the message with the given
    /// sequence number, as well as the number of unread messages remaining in
    /// the queue.
    async fn read_and_delete(
        &self,
        client_id: &QsClientId,
        sequence_number: u64,
        number_of_messages: u64,
    ) -> Result<(Vec<QueueMessage>, u64), Self::ReadAndDeleteError>;
}
