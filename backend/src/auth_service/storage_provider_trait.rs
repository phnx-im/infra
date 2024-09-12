// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{error::Error, fmt::Debug};

use async_trait::async_trait;
use opaque_ke::ServerSetup;
use phnxtypes::{
    credentials::ClientCredential,
    crypto::OpaqueCiphersuite,
    identifiers::{AsClientId, QualifiedUserName},
    messages::{client_as::ConnectionPackage, QueueMessage},
};
use privacypass::batched_tokens_ristretto255::server::BatchedKeyStore;

/// Storage provider trait for the QS.
#[async_trait]
pub trait AsStorageProvider: Sync + Send + 'static {
    type PrivacyPassKeyStore: BatchedKeyStore;
    type StorageError: Error + Debug;

    type StoreClientError: Error + Debug;
    type CreateClientError: Error + Debug;
    type DeleteClientError: Error + Debug;

    type EnqueueError: Error + Debug;
    type ReadAndDeleteError: Error + Debug;

    type StoreKeyPackagesError: Error + Debug;
    type LoadConnectionPackageError: Error + Debug;

    type LoadSigningKeyError: Error + Debug;
    type LoadAsCredentialsError: Error + Debug;

    type LoadOpaqueKeyError: Error + Debug;

    // === Key packages ===

    /// Store connection packages for a specific client.
    async fn store_connection_packages(
        &self,
        client_id: &AsClientId,
        connection_packages: Vec<ConnectionPackage>,
    ) -> Result<(), Self::StoreKeyPackagesError>;

    /// Return a key package for a specific client. The client_id must belong to
    /// the same user as the requested key packages.
    /// TODO: Last resort key package
    async fn client_connection_package(
        &self,
        client_id: &AsClientId,
    ) -> Result<ConnectionPackage, Self::LoadConnectionPackageError>;

    /// Return a key package for each client of a user referenced by a
    /// user name.
    async fn load_user_connection_packages(
        &self,
        user_name: &QualifiedUserName,
    ) -> Result<Vec<ConnectionPackage>, Self::StorageError>;

    // === Messages ===

    // --- Legacy ---

    /// Append the given message to the queue. Returns an error if the payload
    /// is greater than the maximum payload allowed by the storage provider.
    async fn enqueue(
        &self,
        client_id: &AsClientId,
        message: QueueMessage,
    ) -> Result<(), Self::EnqueueError>;

    /// Delete all messages older than the given sequence number in the queue
    /// with the given client ID and return up to the requested number of
    /// messages from the queue starting with the message with the given
    /// sequence number, as well as the number of unread messages remaining in
    /// the queue.
    async fn read_and_delete(
        &self,
        client_id: &AsClientId,
        sequence_number: u64,
        number_of_messages: u64,
    ) -> Result<(Vec<QueueMessage>, u64), Self::ReadAndDeleteError>;

    /// Load the OPAQUE [`ServerSetup`].
    async fn load_opaque_setup(
        &self,
    ) -> Result<ServerSetup<OpaqueCiphersuite>, Self::LoadSigningKeyError>;

    // === Anonymous requests ===

    /// Return the client credentials of a user for a given username.
    async fn client_credentials(&self, user_name: &QualifiedUserName) -> Vec<ClientCredential>;

    // === PrivacyPass ===

    /// Loads the handle of the PrivacyPass keystore.
    async fn privacy_pass_key_store(&self) -> &Self::PrivacyPassKeyStore;

    /// Loads the number of tokens is still allowed to request.
    async fn load_client_token_allowance(
        &self,
        client_id: &AsClientId,
    ) -> Result<usize, Self::StorageError>;

    async fn set_client_token_allowance(
        &self,
        client_id: &AsClientId,
        number_of_tokens: usize,
    ) -> Result<(), Self::StorageError>;

    /// Resets the token allowance of all clients. This should be called after a
    /// rotation of the privacy pass token issuance key material.
    async fn reset_token_allowances(
        &self,
        number_of_tokens: usize,
    ) -> Result<(), Self::StorageError>;
}
