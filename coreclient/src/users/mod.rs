// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
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
    identifiers::{AsClientId, ClientConfig, QsClientId, QsClientReference, QsUserId, UserName},
    messages::{
        client_as::{
            ConnectionEstablishmentPackageTbs, ConnectionPackageTbs, FriendshipPackage,
            UserConnectionPackagesParams,
        },
        FriendshipToken, MlsInfraVersion, QueueMessage,
    },
};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    contacts::{store::ContactStore, Contact, ContactAddInfos, PartialContact},
    conversations::{
        messages::{ConversationMessage, DispatchedConversationMessage, MessageContentType},
        store::{ConversationMessageStore, ConversationStore},
        Conversation, ConversationAttributes,
    },
    groups::store::GroupStore,
    key_stores::{
        as_credentials::AsCredentialStore, leaf_keys::LeafKeyStore,
        qs_verifying_keys::QsVerifyingKeyStore, queue_ratchets::QueueRatchetStore,
        queue_ratchets::QueueType, MemoryUserKeyStore,
    },
    utils::persistence::{open_client_db, open_phnx_db, DataType, Persistable, PersistenceError},
};

use self::{
    api_clients::ApiClients,
    create_user::InitialUserState,
    openmls_provider::PhnxOpenMlsProvider,
    store::{PersistableUserData, UserCreationState},
};

use super::*;

pub(crate) mod api_clients;
mod create_user;
pub(crate) mod openmls_provider;
pub mod process;
pub mod store;
#[cfg(test)]
mod tests;

pub(crate) const CIPHERSUITE: Ciphersuite =
    Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;

pub(crate) const CONNECTION_PACKAGES: usize = 50;
pub(crate) const ADD_PACKAGES: usize = 50;
pub(crate) const CONNECTION_PACKAGE_EXPIRATION_DAYS: i64 = 30;

pub struct SelfUser<T: Notifiable> {
    sqlite_connection: Connection,
    pub(crate) notification_hub_option: Mutex<Option<NotificationHub<T>>>,
    api_clients: ApiClients,
    pub(crate) _qs_user_id: QsUserId,
    pub(crate) qs_client_id: QsClientId,
    pub(crate) key_store: MemoryUserKeyStore,
}

impl<T: Notifiable> SelfUser<T> {
    /// Create a new user with the given `user_name`. If a user with this name
    /// already exists, this will overwrite that user.
    pub async fn new(
        user_name: impl Into<UserName>,
        password: &str,
        server_url: impl ToString,
        notification_hub: NotificationHub<T>,
    ) -> Result<Self> {
        let user_name = user_name.into();
        let as_client_id = AsClientId::random(user_name)?;
        // Open the phnx db to store the client record
        let phnx_db_connection = open_phnx_db()?;

        // Open client specific db
        let client_db_connection = open_client_db(&as_client_id)?;

        Self::new_with_connections(
            as_client_id,
            password,
            server_url,
            notification_hub,
            phnx_db_connection,
            client_db_connection,
        )
        .await
    }

    async fn new_with_connections(
        as_client_id: AsClientId,
        password: &str,
        server_url: impl ToString,
        notification_hub: NotificationHub<T>,
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

        let self_user =
            final_state.into_self_user(client_db_connection, api_clients, Some(notification_hub));

        Ok(self_user)
    }

    /// The same as [`Self::new()`], except that databases ephemeral and dropped
    /// together with this instance of SelfUser.
    //[cfg(debug_assertions)]
    pub async fn new_ephemeral(
        user_name: impl Into<UserName>,
        password: &str,
        server_url: impl ToString,
        notification_hub: NotificationHub<T>,
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
            notification_hub,
            phnx_db_connection,
            client_db_connection,
        )
        .await
    }

    /// Load a user from the database. If a user creation process with a
    /// matching `AsClientId` was interrupted before, this will resume that
    /// process.
    pub async fn load(
        as_client_id: AsClientId,
        notification_hub_option: impl Into<Option<NotificationHub<T>>>,
    ) -> Result<Option<SelfUser<T>>> {
        let phnx_db_connection = open_phnx_db()?;

        let mut client_db_connection = open_client_db(&as_client_id)?;
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

        let self_user =
            final_state.into_self_user(client_db_connection, api_clients, notification_hub_option);

        Ok(Some(self_user))
    }

    /// Create new group
    pub async fn create_conversation(&mut self, title: &str) -> Result<ConversationId> {
        let group_id = self
            .api_clients
            .default_client()?
            .ds_request_group_id()
            .await?;
        let client_reference = self.create_own_client_reference();
        let group_store = self.group_store();
        let (group, partial_params) = group_store.create_group(
            &self.crypto_backend(),
            &self.key_store.signing_key,
            group_id.clone(),
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
        let attributes = ConversationAttributes {
            title: title.to_string(),
        };
        let conversation_store = self.conversation_store();
        let conversation = conversation_store.create_group_conversation(group_id, attributes)?;
        self.dispatch_conversation_notification(conversation.id())?;
        Ok(conversation.id())
    }

    /// Invite users to an existing group
    pub async fn invite_users(
        &mut self,
        conversation_id: ConversationId,
        invited_users: &[UserName],
    ) -> Result<()> {
        let conversation_store = self.conversation_store();
        let conversation = conversation_store
            .get_by_conversation_id(&conversation_id)?
            .ok_or(anyhow!(
                "Can't find conversation with id {}",
                conversation_id.as_uuid()
            ))?;
        let group_id = conversation.group_id().clone();
        let owner_domain = conversation.owner_domain();
        let mut contact_add_infos: Vec<ContactAddInfos> = vec![];
        let mut contact_wai_keys = vec![];
        let mut client_credentials = vec![];
        for invited_user in invited_users {
            let user_name = invited_user.to_string().into();
            let mut contact = self
                .contact_store()
                .get(&user_name)?
                .ok_or(anyhow!("Can't find contact with user name {}", user_name))?;
            contact_wai_keys.push(contact.wai_ear_key().clone());
            client_credentials.push(contact.client_credentials());
            let add_info = self
                .contact_store()
                .add_infos(self.crypto_backend().crypto(), &mut contact)
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
        // We're not getting a response, but if it's not an error, the commit
        // must have gone through.
        self.api_clients
            .get(&owner_domain)?
            .ds_add_users(
                params,
                group.group_state_ear_key(),
                group.user_auth_key().ok_or(anyhow!("No user auth key"))?,
            )
            .await?;

        // Now that we know the commit went through, we can merge the commit and
        // create the events.
        let conversation_messages = group.merge_pending_commit(&self.crypto_backend(), None)?;
        // Send off the notifications
        self.dispatch_message_notifications(conversation_id, conversation_messages)?;
        Ok(())
    }

    pub async fn remove_users(
        &mut self,
        conversation_id: ConversationId,
        target_users: &[UserName],
    ) -> Result<()> {
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
        self.api_clients
            .get(&conversation.owner_domain())?
            .ds_remove_users(
                params,
                group.group_state_ear_key(),
                group.user_auth_key().ok_or(anyhow!("No user auth key"))?,
            )
            .await?;
        // Now that we know the commit went through, we can merge the commit and
        // create the events.
        let conversation_messages = group.merge_pending_commit(&self.crypto_backend(), None)?;
        // Send off the notifications
        self.dispatch_message_notifications(conversation_id, conversation_messages)?;
        Ok(())
    }

    fn dispatch_message_notifications(
        &self,
        conversation_id: ConversationId,
        group_messages: Vec<GroupMessage>,
    ) -> Result<()> {
        let message_store = self.message_store();
        for group_message in group_messages.into_iter() {
            let conversation_message = message_store.create(&conversation_id, group_message)?;
            let dispatched_conversation_message = DispatchedConversationMessage {
                conversation_id,
                conversation_message: conversation_message.clone(),
            };
            // TODO: Unwrapping a mutex poisoning error here for now.
            let mut notification_hub_option = self.notification_hub_option.lock().unwrap();
            if let Some(ref mut notification_hub) = *notification_hub_option {
                notification_hub.dispatch_message_notification(dispatched_conversation_message);
            }
        }
        Ok(())
    }

    fn dispatch_conversation_notification(&self, conversation_id: ConversationId) -> Result<()> {
        // TODO: Unwrapping a mutex poisoning error here for now.
        let mut notification_hub_option = self.notification_hub_option.lock().unwrap();
        if let Some(ref mut notification_hub) = *notification_hub_option {
            notification_hub.dispatch_conversation_notification(conversation_id);
        }
        Ok(())
    }

    /// Send a message and return it. Note that the message has already been
    /// sent to the DS and has internally been stored in the conversation store.
    pub async fn send_message(
        &mut self,
        conversation_id: ConversationId,
        message: MessageContentType,
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
            .create_message(&self.crypto_backend(), message.clone())
            .map_err(CorelibError::Group)?;

        // Send message to DS
        self.api_clients
            .get(&conversation.owner_domain())?
            .ds_send_message(params, group.leaf_signer(), group.group_state_ear_key())
            .await?;

        let conversation_message = self
            .message_store()
            .create(&conversation.id(), message)?
            .into();

        Ok(conversation_message)
    }

    pub async fn add_contact(&mut self, user_name: impl Into<UserName>) -> Result<()> {
        let user_name = user_name.into();
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
        let (connection_group, partial_params) = group_store.create_group(
            &self.crypto_backend(),
            &self.key_store.signing_key,
            group_id.clone(),
        )?;

        // TODO: Once we allow multi-client, invite all our other clients to the
        // connection group.

        let friendship_package = FriendshipPackage {
            friendship_token: self.key_store.friendship_token.clone(),
            add_package_ear_key: self.key_store.add_package_ear_key.clone(),
            client_credential_ear_key: self.key_store.client_credential_ear_key.clone(),
            signature_ear_key_wrapper_key: self.key_store.signature_ear_key_wrapper_key.clone(),
            wai_ear_key: self.key_store.wai_ear_key.clone(),
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
            ConversationAttributes {
                title: user_name.to_string(),
            },
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

        self.dispatch_conversation_notification(conversation.id())?;

        Ok(())
    }

    pub async fn update_user_key(&mut self, conversation_id: ConversationId) -> Result<()> {
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
        self.api_clients
            .get(&owner_domain)?
            .ds_update_client(params, group.group_state_ear_key(), group.leaf_signer())
            .await?;
        let conversation_messages = group.merge_pending_commit(&self.crypto_backend(), None)?;
        self.dispatch_message_notifications(conversation_id, conversation_messages)?;
        Ok(())
    }

    pub async fn delete_group(&mut self, conversation_id: ConversationId) -> Result<()> {
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
        if past_members.len() != 1 {
            let params = group.delete(&self.crypto_backend())?;
            let owner_domain = conversation.owner_domain();
            self.api_clients
                .get(&owner_domain)?
                .ds_delete_group(
                    params,
                    group.user_auth_key().ok_or(anyhow!("No user auth key"))?,
                    group.group_state_ear_key(),
                )
                .await?;
            let conversation_messages = group.merge_pending_commit(&self.crypto_backend(), None)?;
            self.dispatch_message_notifications(conversation_id, conversation_messages)?;
        }
        conversation.set_inactive(&past_members)?;
        Ok(())
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

    pub async fn update(&mut self, conversation_id: ConversationId) -> Result<()> {
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
        self.api_clients
            .get(&owner_domain)?
            .ds_update_client(params, group.group_state_ear_key(), group.leaf_signer())
            .await?;
        let conversation_messages = group.merge_pending_commit(&self.crypto_backend(), None)?;
        self.dispatch_message_notifications(conversation_id, conversation_messages)?;
        Ok(())
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
    pub fn group_members(&self, conversation_id: ConversationId) -> Option<Vec<UserName>> {
        let conversation_store = self.conversation_store();
        let conversation = conversation_store
            .get_by_conversation_id(&conversation_id)
            .ok()??;

        let group_store = self.group_store();
        group_store
            .get(&conversation.group_id())
            .ok()?
            .map(|g| g.members().iter().map(|member| member.clone()).collect())
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
        ContactStore::new(
            &self.sqlite_connection,
            self.qs_verifying_key_store(),
            self.api_clients(),
        )
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

    fn crypto_backend(&self) -> PhnxOpenMlsProvider<'_> {
        PhnxOpenMlsProvider::new(&self.sqlite_connection)
    }
}
