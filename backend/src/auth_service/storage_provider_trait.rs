// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{error::Error, fmt::Debug};

use async_trait::async_trait;
use opaque_ke::{ServerLogin, ServerRegistration, ServerSetup};
use privacypass::batched_tokens::server::BatchedKeyStore;

use crate::{crypto::OpaqueCiphersuite, messages::QueueMessage};

use super::{
    credentials::{
        AsCredential, AsIntermediateCredential, AsIntermediateSigningKey, ClientCredential,
        CredentialFingerprint,
    },
    *,
};

/// Storage provider trait for the QS.
#[async_trait]
pub trait AsStorageProvider: Sync + Send + Debug + 'static {
    type PrivacyPassKeyStore: BatchedKeyStore;
    type StorageError: Error + Debug + PartialEq + Eq + Clone;

    type CreateUserError: Error + Debug + PartialEq + Eq + Clone;
    type StoreUserError: Error + Debug + PartialEq + Eq + Clone;
    type DeleteUserError: Error + Debug + PartialEq + Eq + Clone;

    type StoreClientError: Error + Debug + PartialEq + Eq + Clone;
    type CreateClientError: Error + Debug + PartialEq + Eq + Clone;
    type DeleteClientError: Error + Debug + PartialEq + Eq + Clone;

    type EnqueueError: Error + Debug + PartialEq + Eq + Clone;
    type ReadAndDeleteError: Error + Debug + PartialEq + Eq + Clone;

    type StoreKeyPackagesError: Error + Debug + PartialEq + Eq + Clone;

    type LoadSigningKeyError: Error + Debug + PartialEq + Eq + Clone;
    type LoadAsCredentialsError: Error + Debug + PartialEq + Eq + Clone;

    type LoadOpaqueKeyError: Error + Debug + PartialEq + Eq + Clone;

    // === Users ===

    /// Loads the AsUserRecord for a given UserName. Returns None if no AsUserRecord
    /// exists for the given UserId.
    async fn load_user(
        &self,
        user_name: &UserName,
    ) -> Result<Option<AsUserRecord>, Self::StorageError>;

    /// Create a new user with the given user name. If a user with the given user
    /// name already exists, an error is returned.
    async fn create_user(
        &self,
        user_name: &UserName,
        opaque_record: &ServerRegistration<OpaqueCiphersuite>,
    ) -> Result<AsUserRecord, Self::StorageError>;

    /// Deletes the AsUserRecord for a given UserId. Returns true if a AsUserRecord
    /// was deleted, false if no AsUserRecord existed for the given UserId.
    ///
    /// The storage provider must also delete the following:
    ///  - All clients of the user
    ///  - All enqueued messages for the respective clients
    ///  - All key packages for the respective clients
    async fn delete_user(&self, user_id: &UserName) -> Result<(), Self::DeleteUserError>;

    // --- Legacy ---

    /// Stores a AsUserRecord for a given UserId. If a AsUserRecord already exists
    /// for the given UserId, it will be overwritten.
    async fn store_user(
        &self,
        user_id: &AsUserId,
        user_record: AsUserRecord,
    ) -> Result<(), Self::StoreUserError>;

    // === Clients ===

    /// Load the info for the client with the given client ID.
    async fn load_client(
        &self,
        client_id: &AsClientId,
    ) -> Result<Option<AsClientRecord>, Self::StorageError>;

    /// Saves a client in the storage provider with the given client ID. The
    /// storage provider must associate this client with the user of the client.
    async fn store_client(
        &self,
        client_id: &AsClientId,
        client_record: &AsClientRecord,
    ) -> Result<(), Self::StoreClientError>;

    /// Deletes the client with the given client ID.
    ///
    /// The storage provider must also delete the following:
    ///  - The associated user, if the user has no other clients
    ///  - All enqueued messages for the respective clients
    ///  - All key packages for the respective clients
    async fn delete_client(&self, client_id: &AsClientId) -> Result<(), Self::StorageError>;

    // === Key packages ===

    /// Store key packages for a specific client.
    async fn store_key_packages(
        &self,
        client_id: &AsClientId,
        key_packages: Vec<KeyPackage>,
    ) -> Result<(), Self::StoreKeyPackagesError>;

    /// Return a key package for a specific client. The client_id must belong to
    /// the same user as the requested key packages.
    /// TODO: Last resort key package
    async fn client_key_package(
        &self,
        client_id: &AsClientId,
    ) -> Result<Option<KeyPackage>, Self::StorageError>;

    /// Return a key package for each client of a user referenced by a
    /// user name.
    async fn load_user_key_packages(
        &self,
        user_name: &UserName,
    ) -> Result<Vec<KeyPackage>, Self::StorageError>;

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

    /// Load the currently active signing key and the
    /// [`AsIntermediateCredential`].
    async fn load_signing_key(&self)
        -> Result<AsIntermediateSigningKey, Self::LoadSigningKeyError>;

    /// Load all currently active [`AsCredential`]s and
    /// [`AsIntermediateCredential`]s.
    async fn load_as_credentials(
        &self,
    ) -> Result<
        (
            Vec<AsCredential>,
            Vec<AsIntermediateCredential>,
            Vec<CredentialFingerprint>,
        ),
        Self::LoadAsCredentialsError,
    >;

    /// Load the OPAQUE [`ServerSetup`].
    async fn load_opaque_setup(
        &self,
    ) -> Result<ServerSetup<OpaqueCiphersuite>, Self::LoadSigningKeyError>;

    // === Anonymous requests ===

    /// Return the client credentials of a user for a given username.
    async fn client_credentials(&self, user_name: &UserName) -> Vec<ClientCredential>;

    // === PrivacyPass ===

    /// Loads the handle of the PrivacyPass keystore.
    async fn load_privacy_pass_key_store(
        &self,
    ) -> Result<Self::PrivacyPassKeyStore, Self::StorageError>;

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

#[async_trait]
pub trait AsEphemeralStorageProvider: Sync + Send + Debug + 'static {
    type StorageError: Error + Debug + PartialEq + Eq + Clone;

    /// Store a client credential for a given client ID.
    async fn store_credential(
        &self,
        client_id: AsClientId, // TODO: This is probably redundant, as the ID is contained in the credential.
        credential: &ClientCredential,
    ) -> Result<(), Self::StorageError>;

    /// Load a client credential for a given client ID.
    async fn load_credential(
        &self,
        client_id: &AsClientId,
    ) -> Result<Option<ClientCredential>, Self::StorageError>;

    /// Delete a client credential for a given client ID.
    async fn delete_credential(&self, client_id: &AsClientId) -> Result<(), Self::StorageError>;

    /// Store the login state for a given client ID.
    async fn store_client_login_state(
        &self,
        client_id: AsClientId, // TODO: This is probably redundant, as the ID is contained in the credential.
        credential: &ClientCredential,
        opaque_state: &ServerLogin<OpaqueCiphersuite>,
    ) -> Result<(), Self::StorageError>;

    /// Load the login state for a given client ID.
    async fn load_client_login_state(
        &self,
        client_id: &AsClientId,
    ) -> Result<Option<(ClientCredential, ServerLogin<OpaqueCiphersuite>)>, Self::StorageError>;

    /// Delete the login state for a given client ID.
    async fn delete_client_login_state(
        &self,
        client_id: &AsClientId,
    ) -> Result<(), Self::StorageError>;

    /// Store the login state for a given user name.
    async fn store_user_login_state(
        &self,
        user_name: &UserName,
        opaque_state: &ServerLogin<OpaqueCiphersuite>,
    ) -> Result<(), Self::StorageError>;

    /// Load the login state for a given user name.
    async fn load_user_login_state(
        &self,
        user_name: &UserName,
    ) -> Result<Option<ServerLogin<OpaqueCiphersuite>>, Self::StorageError>;

    /// Delete the login state for a given user name.
    async fn delete_user_login_state(&self, user_name: &UserName)
        -> Result<(), Self::StorageError>;
}