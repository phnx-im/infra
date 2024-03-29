// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    collections::HashSet,
    ops::Deref,
    sync::{Arc, Mutex},
};

use anyhow::{anyhow, Result};
use opaque_ke::{
    ClientRegistration, ClientRegistrationFinishParameters, ClientRegistrationFinishResult,
    ClientRegistrationStartResult, Identifiers, RegistrationUpload,
};
use phnxapiclient::{qs_api::ws::QsWebSocket, ApiClient, ApiClientInitError};
use phnxtypes::{
    credentials::{
        keys::{ClientSigningKey, InfraCredentialSigningKey},
        ClientCredential, ClientCredentialCsr, ClientCredentialPayload,
    },
    crypto::{
        ear::{
            keys::{
                AddPackageEarKey, ClientCredentialEarKey, FriendshipPackageEarKey, PushTokenEarKey,
                SignatureEarKey, SignatureEarKeyWrapperKey, WelcomeAttributionInfoEarKey,
            },
            EarEncryptable,
        },
        hpke::HpkeEncryptable,
        kdf::keys::RatchetSecret,
        signatures::{
            keys::{QsClientSigningKey, QsUserSigningKey},
            signable::{Signable, Verifiable},
        },
        ConnectionDecryptionKey, OpaqueCiphersuite, RatchetDecryptionKey,
    },
    identifiers::{
        AsClientId, ClientConfig, QsClientId, QsClientReference, QsUserId, SafeTryInto, UserName,
    },
    messages::{
        client_as::{ConnectionPackageTbs, UserConnectionPackagesParams},
        FriendshipToken, MlsInfraVersion, QueueMessage,
    },
    time::TimeStamp,
};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    contacts::{store::ContactStore, Contact, ContactAddInfos, PartialContact},
    conversations::{
        messages::ConversationMessage,
        store::{ConversationMessageStore, ConversationStore},
        Conversation, ConversationAttributes,
    },
    groups::store::GroupStore,
    key_stores::{
        as_credentials::AsCredentialStore, leaf_keys::LeafKeyStore,
        qs_verifying_keys::QsVerifyingKeyStore, queue_ratchets::QueueRatchetStore,
        queue_ratchets::QueueType, MemoryUserKeyStore,
    },
    users::{
        connection_establishment::{ConnectionEstablishmentPackageTbs, FriendshipPackage},
        user_profile::UserProfile,
    },
    utils::persistence::{open_client_db, open_phnx_db, DataType, Persistable, PersistenceError},
};

use self::{
    api_clients::ApiClients,
    create_user::InitialUserState,
    groups::TimestampedMessage,
    mimi_content::MimiContent,
    openmls_provider::PhnxOpenMlsProvider,
    store::{PersistableUserData, UserCreationState},
    user_profile::UserProfileStore,
};

use super::*;

pub(crate) mod api_clients;
pub(crate) mod connection_establishment;
mod create_user;
pub(crate) mod openmls_provider;
pub mod process;
pub mod store;
#[cfg(test)]
mod tests;
pub(crate) mod user_profile;

pub(crate) const CIPHERSUITE: Ciphersuite =
    Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;

pub(crate) const CONNECTION_PACKAGES: usize = 50;
pub(crate) const ADD_PACKAGES: usize = 50;
pub(crate) const CONNECTION_PACKAGE_EXPIRATION_DAYS: i64 = 30;

pub struct SelfUser {
    sqlite_connection: Connection,
    api_clients: ApiClients,
    pub(crate) _qs_user_id: QsUserId,
    pub(crate) qs_client_id: QsClientId,
    pub(crate) key_store: MemoryUserKeyStore,
}

impl SelfUser {
    /// Create a new user with the given `user_name`. If a user with this name
    /// already exists, this will overwrite that user.
    pub async fn new(
        user_name: impl SafeTryInto<UserName>,
        password: &str,
        server_url: impl ToString,
        client_db_path: &str,
    ) -> Result<Self> {
        let user_name = user_name.try_into()?;
        let as_client_id = AsClientId::random(user_name)?;
        // Open the phnx db to store the client record
        let phnx_db_connection = open_phnx_db(client_db_path)?;

        // Open client specific db
        let client_db_connection = open_client_db(&as_client_id, client_db_path)?;

        Self::new_with_connections(
            as_client_id,
            password,
            server_url,
            phnx_db_connection,
            client_db_connection,
        )
        .await
    }

    async fn new_with_connections(
        as_client_id: AsClientId,
        password: &str,
        server_url: impl ToString,
        phnx_db_connection: Connection,
        mut client_db_connection: Connection,
    ) -> Result<Self> {
        let server_url = server_url.to_string();
        let api_clients = ApiClients::new(as_client_id.user_name().domain(), server_url.clone());

        let mut client_db_transaction = client_db_connection.transaction()?;

        let user_creation_state = UserCreationState::new(
            &client_db_transaction,
            &phnx_db_connection,
            as_client_id,
            server_url,
            password,
        )?;

        let final_state = user_creation_state
            .complete_user_creation(
                &phnx_db_connection,
                &mut client_db_transaction,
                &api_clients,
            )
            .await?;

        client_db_transaction.commit()?;

        let self_user = final_state.into_self_user(client_db_connection, api_clients);

        Ok(self_user)
    }

    /// The same as [`Self::new()`], except that databases ephemeral and dropped
    /// together with this instance of SelfUser.
    pub async fn new_ephemeral(
        user_name: impl Into<UserName>,
        password: &str,
        server_url: impl ToString,
    ) -> Result<Self> {
        let user_name = user_name.into();
        let as_client_id = AsClientId::random(user_name)?;
        // Open the phnx db to store the client record
        let phnx_db_connection = Connection::open_in_memory()?;

        // Open client specific db
        let client_db_connection = Connection::open_in_memory()?;

        Self::new_with_connections(
            as_client_id,
            password,
            server_url,
            phnx_db_connection,
            client_db_connection,
        )
        .await
    }

    /// Load a user from the database. If a user creation process with a
    /// matching `AsClientId` was interrupted before, this will resume that
    /// process.
    pub async fn load(as_client_id: AsClientId, client_db_path: &str) -> Result<Option<SelfUser>> {
        let phnx_db_connection = open_phnx_db(client_db_path)?;

        let mut client_db_connection = open_client_db(&as_client_id, client_db_path)?;
        let mut client_db_transaction = client_db_connection.transaction()?;

        let Some(user_creation_state) =
            PersistableUserData::load_one(&client_db_transaction, Some(&as_client_id), None)?
        else {
            return Ok(None);
        };

        let api_clients = ApiClients::new(
            as_client_id.user_name().domain(),
            user_creation_state.server_url(),
        );

        let final_state = user_creation_state
            .into_payload()
            .complete_user_creation(
                &phnx_db_connection,
                &mut client_db_transaction,
                &api_clients,
            )
            .await?;

        client_db_transaction.commit()?;

        let self_user = final_state.into_self_user(client_db_connection, api_clients);

        Ok(Some(self_user))
    }

    /// Create new conversation.
    ///
    /// Returns the id of the newly created conversation.
    pub async fn create_conversation(
        &mut self,
        title: &str,
        conversation_picture_option: Option<Vec<u8>>,
    ) -> Result<ConversationId> {
        let group_id = self
            .api_clients
            .default_client()?
            .ds_request_group_id()
            .await?;
        let client_reference = self.create_own_client_reference();
        let group_store = self.group_store();
        // Store the conversation attributes in the group's aad
        let conversation_attributes =
            ConversationAttributes::new(title.to_string(), conversation_picture_option);
        let group_data = serde_json::to_vec(&conversation_attributes)?.into();
        let (group, partial_params) = group_store.create_group(
            &self.crypto_backend(),
            &self.key_store.signing_key,
            group_id.clone(),
            group_data,
        )?;
        let encrypted_client_credential = self
            .key_store
            .signing_key
            .credential()
            .encrypt(group.credential_ear_key())?;
        let params = partial_params.into_params(encrypted_client_credential, client_reference);
        self.api_clients
            .default_client()?
            .ds_create_group(
                params,
                group.group_state_ear_key(),
                group.user_auth_key().ok_or(anyhow!("No user auth key"))?,
            )
            .await?;
        let conversation_store = self.conversation_store();
        let conversation =
            conversation_store.create_group_conversation(group_id, conversation_attributes)?;
        Ok(conversation.id())
    }

    pub async fn store_user_profile(
        &self,
        display_name: String,
        profile_picture_option: Option<Vec<u8>>,
    ) -> Result<()> {
        let user_profile = UserProfile::new(display_name, profile_picture_option);

        let user_profile_store = self.user_profile_store();
        user_profile_store.store(user_profile)?;
        Ok(())
    }

    pub fn set_conversation_picture(
        &self,
        conversation_id: ConversationId,
        conversation_picture_option: Option<Vec<u8>>,
    ) -> Result<()> {
        let conversation_store = self.conversation_store();
        let mut conversation = conversation_store
            .get_by_conversation_id(&conversation_id)?
            .ok_or(anyhow!(
                "Can't find conversation with id {}",
                conversation_id.as_uuid()
            ))?;
        conversation.set_conversation_picture(conversation_picture_option)?;
        Ok(())
    }

    /// Invite users to an existing conversation.
    ///
    /// Since this function causes the creation of an MLS commit, it can cause
    /// more than one effect on the group. As a result this function returns a
    /// vector of [`ConversationMessage`]s that represents the changes to the
    /// group. Note that these returned message have already been persisted.
    pub async fn invite_users(
        &mut self,
        conversation_id: ConversationId,
        invited_users: &[UserName],
    ) -> Result<Vec<ConversationMessage>> {
        let conversation_store = self.conversation_store();
        let conversation = conversation_store
            .get_by_conversation_id(&conversation_id)?
            .ok_or(anyhow!(
                "Can't find conversation with id {}",
                conversation_id.as_uuid()
            ))?;
        let group_id = conversation.group_id().clone();
        let owner_domain = conversation.owner_domain();

        // Fetch fresh KeyPackages and a fresh KeyPackageBatch from the QS for
        // each invited user.
        let mut contact_add_infos: Vec<ContactAddInfos> = vec![];
        let mut contact_wai_keys = vec![];
        let mut client_credentials = vec![];
        for invited_user in invited_users {
            // Get the WAI keys and client credentials for the invited users.
            let contact = self.contact_store().get(invited_user)?.ok_or(anyhow!(
                "Can't find contact with user name {}",
                invited_user
            ))?;
            contact_wai_keys.push(contact.wai_ear_key().clone());
            client_credentials.push(contact.client_credentials());
            let add_info = contact
                .fetch_add_infos(
                    self.api_clients(),
                    self.qs_verifying_key_store(),
                    self.crypto_backend().crypto(),
                )
                .await?;
            contact_add_infos.push(add_info);
        }
        debug_assert!(contact_add_infos.len() == invited_users.len());

        let group_store = self.group_store();
        let mut group = group_store
            .get(&group_id)?
            .ok_or(anyhow!("Can't find group with id {:?}", group_id))?;
        // Adds new member and staged commit
        let params = group.invite(
            &self.crypto_backend(),
            &self.key_store.signing_key,
            contact_add_infos,
            contact_wai_keys,
            client_credentials,
        )?;
        // The DS responds with the timestamp of the commit.
        let ds_timestamp = self
            .api_clients
            .get(&owner_domain)?
            .ds_add_users(
                params,
                group.group_state_ear_key(),
                group.user_auth_key().ok_or(anyhow!("No user auth key"))?,
            )
            .await?;

        // Now that we know the commit went through, we can merge the commit
        let group_messages =
            group.merge_pending_commit(&self.crypto_backend(), None, ds_timestamp)?;
        let conversation_messages = self.store_group_messages(conversation_id, group_messages)?;
        Ok(conversation_messages)
    }

    /// Remove users from the conversation with the given [`ConversationId`].
    ///
    /// Since this function causes the creation of an MLS commit, it can cause
    /// more than one effect on the group. As a result this function returns a
    /// vector of [`ConversationMessage`]s that represents the changes to the
    /// group. Note that these returned message have already been persisted.
    pub async fn remove_users(
        &mut self,
        conversation_id: ConversationId,
        target_users: &[UserName],
    ) -> Result<Vec<ConversationMessage>> {
        let conversation_store = self.conversation_store();
        let conversation = conversation_store
            .get_by_conversation_id(&conversation_id)?
            .ok_or(anyhow!(
                "Can't find conversation with id {}",
                conversation_id.as_uuid()
            ))?;
        let group_id = &conversation.group_id();
        let group_store = self.group_store();
        let mut group = group_store
            .get(group_id)?
            .ok_or(anyhow!("Can't find group with id {:?}", group_id))?;
        let clients = target_users
            .iter()
            .flat_map(|user_name| group.user_client_ids(user_name))
            .collect::<Vec<_>>();
        let params = group.remove(&self.crypto_backend(), clients)?;
        let ds_timestamp = self
            .api_clients
            .get(&conversation.owner_domain())?
            .ds_remove_users(
                params,
                group.group_state_ear_key(),
                group.user_auth_key().ok_or(anyhow!("No user auth key"))?,
            )
            .await?;
        // Now that we know the commit went through, we can merge the commit
        let group_messages =
            group.merge_pending_commit(&self.crypto_backend(), None, ds_timestamp)?;
        let conversation_messages = self.store_group_messages(conversation_id, group_messages)?;
        Ok(conversation_messages)
    }

    /// Send a message and return it. Note that the message has already been
    /// sent to the DS and has internally been stored in the conversation store.
    pub async fn send_message(
        &mut self,
        conversation_id: ConversationId,
        content: MimiContent,
    ) -> Result<ConversationMessage> {
        let conversation_store = self.conversation_store();
        let conversation = conversation_store
            .get_by_conversation_id(&conversation_id)?
            .ok_or(anyhow!(
                "Can't find conversation with id {}",
                conversation_id.as_uuid()
            ))?;
        let group_id = &conversation.group_id();
        // Generate ciphertext
        let group_store = self.group_store();
        let mut group = group_store
            .get(&group_id)?
            .ok_or(anyhow!("Can't find group with id {:?}", group_id))?;
        // Generate ciphertext
        let (params, message) = group
            .create_message(&self.crypto_backend(), content.clone())
            .map_err(CorelibError::Group)?;

        // Send message to DS
        let ds_timestamp = self
            .api_clients
            .get(&conversation.owner_domain())?
            .ds_send_message(params, group.leaf_signer(), group.group_state_ear_key())
            .await?;

        let group_message = TimestampedMessage::from_message_and_timestamp(message, ds_timestamp);
        let conversation_message = self
            .message_store()
            .create(&conversation.id(), group_message)?
            .into();

        Ok(conversation_message)
    }

    /// Create a connection with a new user.
    ///
    /// Returns the [`ConversationId`] of the newly created connection
    /// conversation.
    pub async fn add_contact(
        &mut self,
        user_name: impl SafeTryInto<UserName>,
    ) -> Result<ConversationId> {
        let user_name = user_name.try_into()?;
        let params = UserConnectionPackagesParams {
            user_name: user_name.clone(),
        };
        // First we fetch connection key packages from the AS, then we establish
        // a connection group. Finally, we fully add the user as a contact.
        let user_domain = user_name.domain();
        log::info!("Adding contact {}", user_name);
        let user_key_packages = self
            .api_clients
            .get(&user_domain)?
            .as_user_connection_packages(params)
            .await?;

        // The AS should return an error if the user does not exist, but we
        // check here locally just to be sure.
        if user_key_packages.connection_packages.is_empty() {
            return Err(anyhow!("User {} does not exist", user_name));
        }
        // Verify the connection key packages
        log::info!("Verifying connection packages");
        let mut verified_connection_packages = vec![];
        let as_credential_store = self.as_credential_store();
        for connection_package in user_key_packages.connection_packages.into_iter() {
            let as_intermediate_credential = as_credential_store
                .get(
                    &user_domain,
                    connection_package.client_credential_signer_fingerprint(),
                )
                .await?;
            let verifying_key = as_intermediate_credential.verifying_key();
            verified_connection_packages.push(connection_package.verify(verifying_key)?)
        }

        // TODO: Connection Package Validation
        // * Version
        // * Lifetime

        // Get a group id for the connection group
        log::info!("Requesting group id");
        let group_id = self
            .api_clients
            .default_client()?
            .ds_request_group_id()
            .await?;
        // Create the connection group
        log::info!("Creating local connection group");
        let group_store = self.group_store();
        let title = format!("Connection group: {} - {}", self.user_name(), user_name);
        let conversation_attributes = ConversationAttributes::new(title.to_string(), None);
        let group_data = serde_json::to_vec(&conversation_attributes)?.into();
        let (connection_group, partial_params) = group_store.create_group(
            &self.crypto_backend(),
            &self.key_store.signing_key,
            group_id.clone(),
            group_data,
        )?;

        // TODO: Once we allow multi-client, invite all our other clients to the
        // connection group.
        let user_profile = self
            .user_profile_store()
            .get()?
            .unwrap_or(UserProfile::from(self.user_name()));

        let friendship_package = FriendshipPackage {
            friendship_token: self.key_store.friendship_token.clone(),
            add_package_ear_key: self.key_store.add_package_ear_key.clone(),
            client_credential_ear_key: self.key_store.client_credential_ear_key.clone(),
            signature_ear_key_wrapper_key: self.key_store.signature_ear_key_wrapper_key.clone(),
            wai_ear_key: self.key_store.wai_ear_key.clone(),
            user_profile,
        };

        let friendship_package_ear_key = FriendshipPackageEarKey::random()?;

        // Create a connection establishment package
        let connection_establishment_package = ConnectionEstablishmentPackageTbs {
            sender_client_credential: self.key_store.signing_key.credential().clone(),
            connection_group_id: group_id.clone(),
            connection_group_ear_key: connection_group.group_state_ear_key().clone(),
            connection_group_credential_key: connection_group.credential_ear_key().clone(),
            connection_group_signature_ear_key_wrapper_key: connection_group
                .signature_ear_key_wrapper_key()
                .clone(),
            friendship_package_ear_key: friendship_package_ear_key.clone(),
            friendship_package,
        }
        .sign(&self.key_store.signing_key)?;

        let client_reference = self.create_own_client_reference();
        let encrypted_client_credential = self
            .key_store
            .signing_key
            .credential()
            .encrypt(connection_group.credential_ear_key())?;
        let params = partial_params.into_params(encrypted_client_credential, client_reference);
        log::info!("Creating connection group on DS");
        self.api_clients
            .default_client()?
            .ds_create_group(
                params,
                connection_group.group_state_ear_key(),
                connection_group
                    .user_auth_key()
                    .ok_or(anyhow!("No user auth key"))?,
            )
            .await?;

        // Create the connection conversation
        let conversation_store = self.conversation_store();
        let conversation = conversation_store.create_connection_conversation(
            group_id,
            user_name.clone(),
            conversation_attributes,
        )?;

        // Create and persist a new partial contact
        self.contact_store().store_partial_contact(
            &user_name,
            &conversation.id(),
            friendship_package_ear_key,
        )?;

        // Encrypt the connection establishment package for each connection and send it off.
        for connection_package in verified_connection_packages {
            let ciphertext = connection_establishment_package.encrypt(
                connection_package.encryption_key(),
                &[],
                &[],
            );
            let client_id = connection_package.client_credential().identity();

            self.api_clients
                .get(&user_domain)?
                .as_enqueue_message(client_id, ciphertext)
                .await?;
        }

        Ok(conversation.id())
    }

    /// Update the user's user auth key in the conversation with the given
    /// [`ConversationId`].
    ///
    /// Since this function causes the creation of an MLS commit, it can cause
    /// more than one effect on the group. As a result this function returns a
    /// vector of [`ConversationMessage`]s that represents the changes to the
    /// group. Note that these returned message have already been persisted.
    pub async fn update_user_key(
        &mut self,
        conversation_id: ConversationId,
    ) -> Result<Vec<ConversationMessage>> {
        let conversation_store = self.conversation_store();
        let conversation = conversation_store
            .get_by_conversation_id(&conversation_id)?
            .ok_or(anyhow!(
                "Can't find conversation with id {}",
                conversation_id.as_uuid()
            ))?;
        let group_id = conversation.group_id();
        // Generate ciphertext
        let group_store = self.group_store();
        let mut group = group_store
            .get(&group_id)?
            .ok_or(anyhow!("Can't find group with id {:?}", group_id))?;
        let params = group.update_user_key(&self.crypto_backend())?;
        let owner_domain = conversation.owner_domain();
        let ds_timestamp = self
            .api_clients
            .get(&owner_domain)?
            .ds_update_client(params, group.group_state_ear_key(), group.leaf_signer())
            .await?;
        let group_messages =
            group.merge_pending_commit(&self.crypto_backend(), None, ds_timestamp)?;
        let conversation_messages = self.store_group_messages(conversation_id, group_messages)?;
        Ok(conversation_messages)
    }

    /// Delete the conversation with the given [`ConversationId`].
    ///
    /// Since this function causes the creation of an MLS commit, it can cause
    /// more than one effect on the group. As a result this function returns a
    /// vector of [`ConversationMessage`]s that represents the changes to the
    /// group. Note that these returned message have already been persisted.
    pub async fn delete_group(
        &mut self,
        conversation_id: ConversationId,
    ) -> Result<Vec<ConversationMessage>> {
        let conversation_store = self.conversation_store();
        let mut conversation = conversation_store
            .get_by_conversation_id(&conversation_id)?
            .ok_or(anyhow!(
                "Can't find conversation with id {}",
                conversation_id.as_uuid()
            ))?;
        let group_id = conversation.group_id();
        // Generate ciphertext
        let group_store = self.group_store();
        let mut group = group_store
            .get(&group_id)?
            .ok_or(anyhow!("Can't find group with id {:?}", group_id))?;
        let past_members = group.members();
        // No need to send a message to the server if we are the only member.
        // TODO: Make sure this is what we want.
        let group_messages = if past_members.len() != 1 {
            let params = group.delete(&self.crypto_backend())?;
            let owner_domain = conversation.owner_domain();
            let ds_timestamp = self
                .api_clients
                .get(&owner_domain)?
                .ds_delete_group(
                    params,
                    group.user_auth_key().ok_or(anyhow!("No user auth key"))?,
                    group.group_state_ear_key(),
                )
                .await?;
            group.merge_pending_commit(&self.crypto_backend(), None, ds_timestamp)?
        } else {
            vec![]
        };
        conversation.set_inactive(past_members.into_iter().collect())?;
        let conversation_messages = self.store_group_messages(conversation_id, group_messages)?;
        Ok(conversation_messages)
    }

    async fn fetch_messages_from_queue(
        &mut self,
        queue_type: QueueType,
    ) -> Result<Vec<QueueMessage>> {
        let mut remaining_messages = 1;
        let mut messages: Vec<QueueMessage> = Vec::new();
        let queue_ratchet_store = self.queue_ratchet_store();
        let mut sequence_number = queue_ratchet_store.get_sequence_number(queue_type)?;
        while remaining_messages > 0 {
            let api_client = self.api_clients.default_client()?;
            let mut response = match &queue_type {
                QueueType::As => {
                    api_client
                        .as_dequeue_messages(
                            **sequence_number,
                            1_000_000,
                            &self.key_store.signing_key,
                        )
                        .await?
                }
                QueueType::Qs => {
                    api_client
                        .qs_dequeue_messages(
                            &self.qs_client_id,
                            **sequence_number,
                            1_000_000,
                            &self.key_store.qs_client_signing_key,
                        )
                        .await?
                }
            };

            if let Some(message) = messages.last() {
                sequence_number.set(message.sequence_number)?;
            }

            remaining_messages = response.remaining_messages_number;
            messages.append(&mut response.messages);

            if let Some(message) = messages.last() {
                sequence_number.set(message.sequence_number + 1)?;
            }
        }
        Ok(messages)
    }

    pub async fn as_fetch_messages(&mut self) -> Result<Vec<QueueMessage>> {
        self.fetch_messages_from_queue(QueueType::As).await
    }

    pub async fn qs_fetch_messages(&mut self) -> Result<Vec<QueueMessage>> {
        self.fetch_messages_from_queue(QueueType::Qs).await
    }

    pub async fn leave_group(&mut self, conversation_id: ConversationId) -> Result<()> {
        let conversation_store = self.conversation_store();
        let conversation = conversation_store
            .get_by_conversation_id(&conversation_id)?
            .ok_or(anyhow!(
                "Can't find conversation with id {}",
                conversation_id.as_uuid()
            ))?;
        let group_id = conversation.group_id();
        let group_store = self.group_store();
        let mut group = group_store
            .get(&group_id)?
            .ok_or(anyhow!("Can't find group with id {:?}", group_id))?;
        let params = group.leave_group(&self.crypto_backend())?;
        let owner_domain = conversation.owner_domain();
        self.api_clients
            .get(&owner_domain)?
            .ds_self_remove_client(
                params,
                group.user_auth_key().ok_or(anyhow!("No user auth key"))?,
                group.group_state_ear_key(),
            )
            .await?;
        Ok(())
    }

    /// Update the user's key material in the conversation with the given
    /// [`ConversationId`].
    ///
    /// Since this function causes the creation of an MLS commit, it can cause
    /// more than one effect on the group. As a result this function returns a
    /// vector of [`ConversationMessage`]s that represents the changes to the
    /// group. Note that these returned message have already been persisted.
    pub async fn update(
        &mut self,
        conversation_id: ConversationId,
    ) -> Result<Vec<ConversationMessage>> {
        let conversation_store = self.conversation_store();
        let conversation = conversation_store
            .get_by_conversation_id(&conversation_id)?
            .ok_or(anyhow!(
                "Can't find conversation with id {}",
                conversation_id.as_uuid()
            ))?;
        let group_id = conversation.group_id();
        let group_store = self.group_store();
        let mut group = group_store
            .get(&group_id)?
            .ok_or(anyhow!("Can't find group with id {:?}", group_id))?;
        let params = group.update(&self.crypto_backend())?;
        let owner_domain = conversation.owner_domain();
        let ds_timestamp = self
            .api_clients
            .get(&owner_domain)?
            .ds_update_client(params, group.group_state_ear_key(), group.leaf_signer())
            .await?;
        let group_messages =
            group.merge_pending_commit(&self.crypto_backend(), None, ds_timestamp)?;
        let conversation_messages = self.store_group_messages(conversation_id, group_messages)?;
        Ok(conversation_messages)
    }

    pub fn contacts(&self) -> Result<Vec<Contact>, PersistenceError> {
        let contact_store = self.contact_store();
        contact_store.get_all_contacts().map(|cs| {
            cs.into_iter()
                .map(|c| c.convert_for_export())
                .collect::<Vec<_>>()
        })
    }

    pub fn partial_contacts(&self) -> Result<Vec<PartialContact>, PersistenceError> {
        let contact_store = self.contact_store();
        contact_store.get_all_partial_contacts().map(|cs| {
            cs.into_iter()
                .map(|c| c.convert_for_export())
                .collect::<Vec<_>>()
        })
    }

    fn create_own_client_reference(&self) -> QsClientReference {
        let sealed_reference = ClientConfig {
            client_id: self.qs_client_id.clone(),
            push_token_ear_key: Some(self.key_store.push_token_ear_key.clone()),
        }
        .encrypt(&self.key_store.qs_client_id_encryption_key, &[], &[]);
        QsClientReference {
            client_homeserver_domain: self.user_name().domain(),
            sealed_reference,
        }
    }

    pub fn user_name(&self) -> UserName {
        self.key_store
            .signing_key
            .credential()
            .identity()
            .user_name()
    }

    /// Returns None if there is no conversation with the given id.
    pub fn group_members(&self, conversation_id: ConversationId) -> Option<HashSet<UserName>> {
        let conversation_store = self.conversation_store();
        let conversation = conversation_store
            .get_by_conversation_id(&conversation_id)
            .ok()??;

        let group_store = self.group_store();
        group_store
            .get(&conversation.group_id())
            .ok()?
            .map(|g| g.members())
    }

    pub fn pending_removes(&self, conversation_id: ConversationId) -> Option<Vec<UserName>> {
        let conversation_store = self.conversation_store();
        let conversation = conversation_store
            .get_by_conversation_id(&conversation_id)
            .ok()??;

        let group_store = self.group_store();
        group_store
            .get(&conversation.group_id())
            .ok()?
            .map(|group| group.pending_removes())
    }

    pub fn conversations(&self) -> Result<Vec<Conversation>, PersistenceError> {
        let conversation_store = self.conversation_store();
        conversation_store
            .get_all()
            .map(|cs| cs.into_iter().map(|c| c.convert_for_export()).collect())
    }

    pub async fn websocket(&mut self, timeout: u64, retry_interval: u64) -> Result<QsWebSocket> {
        let api_client = self.api_clients.default_client();
        Ok(api_client?
            .spawn_websocket(self.qs_client_id.clone(), timeout, retry_interval)
            .await?)
    }

    /// Mark all messages in the conversation with the given conversation id and
    /// with a timestamp older than the given timestamp as read.
    pub fn mark_as_read<'b, T: 'b + IntoIterator<Item = (&'b ConversationId, &'b TimeStamp)>>(
        &self,
        mark_as_read_data: T,
    ) -> Result<(), PersistenceError> {
        let conversation_store = self.conversation_store();
        conversation_store.mark_as_read(mark_as_read_data)
    }

    /// Returns how many messages in the conversation with the given ID are
    /// marked as unread.
    pub fn unread_message_count(
        &self,
        conversation_id: ConversationId,
    ) -> Result<u32, PersistenceError> {
        let conversation_store = self.conversation_store();
        conversation_store.unread_message_count(conversation_id)
    }

    pub fn as_client_id(&self) -> AsClientId {
        self.key_store.signing_key.credential().identity().clone()
    }

    fn store_group_messages(
        &self,
        conversation_id: ConversationId,
        group_messages: Vec<TimestampedMessage>,
    ) -> Result<Vec<ConversationMessage>> {
        let message_store = self.message_store();
        let mut stored_messages = vec![];
        // TODO: This should be done as part of a transaction
        for group_message in group_messages.into_iter() {
            let stored_message = message_store.create(&conversation_id, group_message)?;
            stored_messages.push(stored_message.payload);
        }
        Ok(stored_messages)
    }

    fn api_clients(&self) -> ApiClients {
        self.api_clients.clone()
    }

    fn conversation_store(&self) -> ConversationStore<'_> {
        (&self.sqlite_connection).into()
    }

    fn qs_verifying_key_store(&self) -> QsVerifyingKeyStore<'_> {
        QsVerifyingKeyStore::new(&self.sqlite_connection, self.api_clients())
    }

    fn as_credential_store(&self) -> AsCredentialStore<'_> {
        AsCredentialStore::new(&self.sqlite_connection, self.api_clients())
    }

    fn contact_store(&self) -> ContactStore<'_> {
        ContactStore::new(&self.sqlite_connection)
    }

    fn group_store(&self) -> GroupStore<'_> {
        (&self.sqlite_connection).into()
    }

    fn message_store(&self) -> ConversationMessageStore<'_> {
        (&self.sqlite_connection).into()
    }

    fn queue_ratchet_store(&self) -> QueueRatchetStore<'_> {
        (&self.sqlite_connection).into()
    }

    fn leaf_key_store(&self) -> LeafKeyStore<'_> {
        (&self.sqlite_connection).into()
    }

    fn user_profile_store(&self) -> UserProfileStore<'_> {
        (&self.sqlite_connection).into()
    }

    fn crypto_backend(&self) -> PhnxOpenMlsProvider<'_> {
        PhnxOpenMlsProvider::new(&self.sqlite_connection)
    }
}
