// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{collections::HashSet, sync::Arc};

use anyhow::{anyhow, bail, Result};
use exif::{Reader, Tag};
use groups::{
    client_auth_info::StorableClientCredential, openmls_provider::PhnxOpenMlsProvider, Group,
};
use key_stores::as_credentials::AsCredentials;
use opaque_ke::{
    ClientRegistration, ClientRegistrationFinishParameters, ClientRegistrationFinishResult,
    ClientRegistrationStartResult, Identifiers, RegistrationUpload,
};
use own_client_info::OwnClientInfo;
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
use rusqlite::{Connection, Transaction};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::Mutex;
use utils::set_up_database;
use uuid::Uuid;

use crate::{
    clients::connection_establishment::{ConnectionEstablishmentPackageTbs, FriendshipPackage},
    contacts::{Contact, ContactAddInfos, PartialContact},
    conversations::{messages::ConversationMessage, Conversation, ConversationAttributes},
    key_stores::{queue_ratchets::QueueType, MemoryUserKeyStore},
    user_profiles::UserProfile,
    utils::persistence::{open_client_db, open_phnx_db, DataType, Persistable, PersistenceError},
};

use self::{
    api_clients::ApiClients,
    conversations::messages::TimestampedMessage,
    create_user::InitialUserState,
    mimi_content::MimiContent,
    store::{PersistableUserData, UserCreationState},
};

use super::*;

pub(crate) mod api_clients;
pub(crate) mod connection_establishment;
mod create_user;
pub(crate) mod own_client_info;
pub mod process;
pub mod store;
#[cfg(test)]
mod tests;

pub(crate) const CIPHERSUITE: Ciphersuite =
    Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;

pub(crate) const CONNECTION_PACKAGES: usize = 50;
pub(crate) const ADD_PACKAGES: usize = 50;
pub(crate) const CONNECTION_PACKAGE_EXPIRATION_DAYS: i64 = 30;

#[derive(Clone)]
pub struct CoreUser {
    sqlite_connection: Arc<Mutex<Connection>>,
    api_clients: ApiClients,
    pub(crate) _qs_user_id: QsUserId,
    pub(crate) qs_client_id: QsClientId,
    pub(crate) key_store: MemoryUserKeyStore,
}

impl CoreUser {
    /// Create a new user with the given `user_name`. If a user with this name
    /// already exists, this will overwrite that user.
    pub async fn new(
        user_name: impl SafeTryInto<UserName>,
        password: &str,
        server_url: impl ToString,
        db_path: &str,
    ) -> Result<Self> {
        let user_name = user_name.try_into()?;
        let as_client_id = AsClientId::random(user_name)?;
        // Open the phnx db to store the client record
        let phnx_db_connection = open_phnx_db(db_path)?;

        // Open client specific db
        let client_db_connection = open_client_db(&as_client_id, db_path)?;

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

        set_up_database(&mut client_db_connection)?;

        let mut client_db_transaction = client_db_connection.transaction()?;

        let user_creation_state = UserCreationState::new(
            &client_db_transaction,
            &phnx_db_connection,
            as_client_id,
            server_url.clone(),
            password,
        )?;

        let final_state = user_creation_state
            .complete_user_creation(
                &phnx_db_connection,
                &mut client_db_transaction,
                &api_clients,
            )
            .await?;

        OwnClientInfo {
            server_url,
            qs_user_id: final_state.qs_user_id().clone(),
            qs_client_id: final_state.qs_client_id().clone(),
            as_client_id: final_state.client_id().clone(),
        }
        .store(&client_db_transaction)?;

        client_db_transaction.commit()?;

        let self_user = final_state.into_self_user(client_db_connection, api_clients);

        Ok(self_user)
    }

    /// The same as [`Self::new()`], except that databases ephemeral and dropped
    /// together with this instance of CoreUser.
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
    pub async fn load(as_client_id: AsClientId, db_path: &str) -> Result<Option<CoreUser>> {
        let phnx_db_connection = open_phnx_db(db_path)?;

        let mut client_db_connection = open_client_db(&as_client_id, db_path)?;

        set_up_database(&mut client_db_connection)?;

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
        &self,
        title: &str,
        conversation_picture_option: Option<Vec<u8>>,
    ) -> Result<ConversationId> {
        let group_id = self
            .api_clients
            .default_client()?
            .ds_request_group_id()
            .await?;
        let client_reference = self.create_own_client_reference();
        // Store the conversation attributes in the group's aad
        let conversation_attributes =
            ConversationAttributes::new(title.to_string(), conversation_picture_option);
        let group_data = serde_json::to_vec(&conversation_attributes)?.into();
        let connection = self.sqlite_connection.lock().await;
        let (group, partial_params) = Group::create_group(
            &connection,
            &self.key_store.signing_key,
            group_id.clone(),
            group_data,
        )?;
        drop(connection);
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

        let connection = self.sqlite_connection.lock().await;
        group.store(&connection)?;
        let conversation = Conversation::new_group_conversation(group_id, conversation_attributes);
        conversation.store(&connection)?;
        Ok(conversation.id())
    }

    pub async fn set_own_user_profile(&self, mut user_profile: UserProfile) -> Result<()> {
        if user_profile.user_name() != &self.user_name() {
            bail!("Can't set user profile for users other than the current user.",);
        }
        if let Some(profile_picture) = user_profile.profile_picture() {
            let new_image = match profile_picture {
                Asset::Value(image_bytes) => self.resize_image(image_bytes)?,
            };
            user_profile.set_profile_picture(Some(Asset::Value(new_image)));
        }
        let connection = &self.sqlite_connection.lock().await;
        user_profile.update(connection)?;
        Ok(())
    }

    /// Get the user profile of the user with the given [`UserName`].
    pub async fn user_profile(&self, user_name: &UserName) -> Result<Option<UserProfile>> {
        let connection = &self.sqlite_connection.lock().await;
        let user = UserProfile::load(connection, user_name)?;
        Ok(user)
    }

    pub async fn set_conversation_picture(
        &self,
        conversation_id: ConversationId,
        conversation_picture_option: Option<Vec<u8>>,
    ) -> Result<()> {
        let connection = &self.sqlite_connection.lock().await;
        let mut conversation = Conversation::load(connection, &conversation_id)?.ok_or(anyhow!(
            "Can't find conversation with id {}",
            conversation_id.as_uuid()
        ))?;
        let resized_picture_option = conversation_picture_option
            .and_then(|conversation_picture| self.resize_image(&conversation_picture).ok());
        conversation.set_conversation_picture(connection, resized_picture_option)?;
        Ok(())
    }

    fn resize_image(&self, mut image_bytes: &[u8]) -> Result<Vec<u8>> {
        let image = image::load_from_memory(image_bytes)?;

        // Read EXIF data
        let exif_reader = Reader::new();
        let mut image_bytes_cursor = std::io::Cursor::new(&mut image_bytes);
        let exif = exif_reader
            .read_from_container(&mut image_bytes_cursor)
            .ok();

        // Resize the image
        let image = image.resize(256, 256, image::imageops::FilterType::Nearest);

        // Rotate/flip the image according to the orientation if necessary
        let image = if let Some(exif) = exif {
            let orientation = exif
                .get_field(Tag::Orientation, exif::In::PRIMARY)
                .and_then(|field| field.value.get_uint(0))
                .unwrap_or(1);
            match orientation {
                1 => image,
                2 => image.fliph(),
                3 => image.rotate180(),
                4 => image.flipv(),
                5 => image.rotate90().fliph(),
                6 => image.rotate90(),
                7 => image.rotate270().fliph(),
                8 => image.rotate270(),
                _ => image,
            }
        } else {
            image
        };

        // Save the resized image
        let mut buf = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut buf);
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, 90);
        encoder.encode_image(&image)?;
        log::info!(
            "Resized profile picture from {} to {} bytes",
            image_bytes.len(),
            buf.len()
        );
        Ok(buf)
    }

    /// Invite users to an existing conversation.
    ///
    /// Since this function causes the creation of an MLS commit, it can cause
    /// more than one effect on the group. As a result this function returns a
    /// vector of [`ConversationMessage`]s that represents the changes to the
    /// group. Note that these returned message have already been persisted.
    pub async fn invite_users(
        &self,
        conversation_id: ConversationId,
        invited_users: &[UserName],
    ) -> Result<Vec<ConversationMessage>> {
        let mut connection = self.sqlite_connection.lock().await;
        let mut transaction = connection.transaction()?;
        let conversation = Conversation::load(&transaction, &conversation_id)?.ok_or(anyhow!(
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
            let contact = Contact::load(&transaction, invited_user)?.ok_or(anyhow!(
                "Can't find contact with user name {}",
                invited_user
            ))?;
            contact_wai_keys.push(contact.wai_ear_key().clone());
            let contact_client_credentials = contact
                .clients()
                .iter()
                .filter_map(|client_id| {
                    match StorableClientCredential::load_by_client_id(&transaction, client_id) {
                        Ok(Some(client_credential)) => {
                            Some(Ok(ClientCredential::from(client_credential)))
                        }
                        Ok(None) => None,
                        Err(e) => Some(Err(e)),
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            client_credentials.push(contact_client_credentials);
            let add_info = contact
                .fetch_add_infos(&transaction, self.api_clients())
                .await?;
            contact_add_infos.push(add_info);
        }
        debug_assert!(contact_add_infos.len() == invited_users.len());

        let mut group = Group::load(&transaction, &group_id)?
            .ok_or(anyhow!("Can't find group with id {:?}", group_id))?;
        // Adds new member and staged commit
        let params = group.invite(
            &transaction,
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
        let group_messages = group.merge_pending_commit(&transaction, None, ds_timestamp)?;
        group.store_update(&transaction)?;

        let conversation_messages =
            Self::store_messages(&mut transaction, conversation_id, group_messages)?;
        transaction.commit()?;
        Ok(conversation_messages)
    }

    /// Remove users from the conversation with the given [`ConversationId`].
    ///
    /// Since this function causes the creation of an MLS commit, it can cause
    /// more than one effect on the group. As a result this function returns a
    /// vector of [`ConversationMessage`]s that represents the changes to the
    /// group. Note that these returned message have already been persisted.
    pub async fn remove_users(
        &self,
        conversation_id: ConversationId,
        target_users: &[UserName],
    ) -> Result<Vec<ConversationMessage>> {
        let mut connection = self.sqlite_connection.lock().await;
        let mut transaction = connection.transaction()?;
        let conversation = Conversation::load(&transaction, &conversation_id)?.ok_or(anyhow!(
            "Can't find conversation with id {}",
            conversation_id.as_uuid()
        ))?;
        let group_id = conversation.group_id();
        let mut group = Group::load(&transaction, group_id)?
            .ok_or(anyhow!("Can't find group with id {:?}", group_id))?;
        let clients = target_users
            .iter()
            .flat_map(|user_name| group.user_client_ids(&transaction, user_name))
            .collect::<Vec<_>>();
        let params = group.remove(&transaction, clients)?;
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
        let group_messages = group.merge_pending_commit(&transaction, None, ds_timestamp)?;
        group.store_update(&transaction)?;

        let conversation_messages =
            Self::store_messages(&mut transaction, conversation_id, group_messages)?;
        transaction.commit()?;
        Ok(conversation_messages)
    }

    /// Send a message and return it. Note that the message has already been
    /// sent to the DS and has internally been stored in the conversation store.
    pub async fn send_message(
        &self,
        conversation_id: ConversationId,
        content: MimiContent,
    ) -> Result<ConversationMessage> {
        let connection = &self.sqlite_connection.lock().await;
        let conversation = Conversation::load(connection, &conversation_id)?.ok_or(anyhow!(
            "Can't find conversation with id {}",
            conversation_id.as_uuid()
        ))?;
        let group_id = conversation.group_id();
        // Store the message as unsent so that we don't lose it in case
        // something goes wrong.
        let mut conversation_message = ConversationMessage::new_unsent_message(
            self.user_name().to_string(),
            conversation_id,
            content.clone(),
        );
        conversation_message.store(connection)?;
        let mut group = Group::load(connection, group_id)?
            .ok_or(anyhow!("Can't find group with id {:?}", group_id))?;
        let params = group
            .create_message(connection, content)
            .map_err(CorelibError::Group)?;

        // Send message to DS
        let ds_timestamp = self
            .api_clients
            .get(&conversation.owner_domain())?
            .ds_send_message(params, group.leaf_signer(), group.group_state_ear_key())
            .await?;

        group.store_update(connection)?;

        // Mark the message as sent.
        conversation_message.mark_as_sent(connection, ds_timestamp)?;

        Ok(conversation_message)
    }

    /// Re-try sending a message, where sending previously failed.
    pub async fn re_send_message(&mut self, local_message_id: Uuid) -> Result<()> {
        let connection = &self.sqlite_connection.lock().await;
        let mut unsent_message = ConversationMessage::load(connection, &local_message_id)?.ok_or(
            anyhow!("Can't find unsent message with id {}", local_message_id),
        )?;
        let content = match unsent_message.message() {
            Message::Content(content_message) if !content_message.was_sent() => {
                content_message.content().clone()
            }
            _ => bail!("Message with id {} was already sent", local_message_id),
        };
        let conversation_id = unsent_message.conversation_id();
        let conversation = Conversation::load(connection, &conversation_id)?.ok_or(anyhow!(
            "Can't find conversation with id {}",
            conversation_id.as_uuid()
        ))?;
        let group_id = conversation.group_id();
        let mut group = Group::load(connection, group_id)?
            .ok_or(anyhow!("Can't find group with id {:?}", group_id))?;
        let params = group
            .create_message(connection, content)
            .map_err(CorelibError::Group)?;

        // Send message to DS
        let ds_timestamp = self
            .api_clients
            .get(&conversation.owner_domain())?
            .ds_send_message(params, group.leaf_signer(), group.group_state_ear_key())
            .await?;

        group.store_update(connection)?;

        // Mark the message as sent.
        unsent_message.mark_as_sent(connection, ds_timestamp)?;

        Ok(())
    }

    /// Create a connection with a new user.
    ///
    /// Returns the [`ConversationId`] of the newly created connection
    /// conversation.
    pub async fn add_contact(
        &self,
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
        let connection = &self.sqlite_connection.lock().await;
        for connection_package in user_key_packages.connection_packages.into_iter() {
            let as_intermediate_credential = AsCredentials::get(
                connection,
                &self.api_clients,
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
        let title = format!("Connection group: {} - {}", self.user_name(), user_name);
        let conversation_attributes = ConversationAttributes::new(title.to_string(), None);
        let group_data = serde_json::to_vec(&conversation_attributes)?.into();
        let (connection_group, partial_params) = Group::create_group(
            connection,
            &self.key_store.signing_key,
            group_id.clone(),
            group_data,
        )?;

        // TODO: Once we allow multi-client, invite all our other clients to the
        // connection group.

        let own_user_profile = UserProfile::load(connection, &self.user_name())
            // We unwrap here, because we know that the user exists.
            .map(|user_option| user_option.unwrap())?;

        let friendship_package = FriendshipPackage {
            friendship_token: self.key_store.friendship_token.clone(),
            add_package_ear_key: self.key_store.add_package_ear_key.clone(),
            client_credential_ear_key: self.key_store.client_credential_ear_key.clone(),
            signature_ear_key_wrapper_key: self.key_store.signature_ear_key_wrapper_key.clone(),
            wai_ear_key: self.key_store.wai_ear_key.clone(),
            user_profile: own_user_profile,
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

        connection_group.store(connection)?;

        // Create the connection conversation
        let conversation = Conversation::new_connection_conversation(
            group_id,
            user_name.clone(),
            conversation_attributes,
        )?;
        conversation.store(connection)?;

        // Create and persist a new partial contact
        PartialContact::new(
            user_name.clone(),
            conversation.id(),
            friendship_package_ear_key,
        )
        .store(connection)?;

        // Store the user profile of the partial contact (we don't have a
        // display name or a profile picture yet)
        UserProfile::new(user_name, None, None).store(connection)?;

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
        &self,
        conversation_id: ConversationId,
    ) -> Result<Vec<ConversationMessage>> {
        let mut connection = self.sqlite_connection.lock().await;
        let mut transaction = connection.transaction()?;
        let conversation = Conversation::load(&transaction, &conversation_id)?.ok_or(anyhow!(
            "Can't find conversation with id {}",
            conversation_id.as_uuid()
        ))?;
        let group_id = conversation.group_id();
        // Generate ciphertext
        let mut group = Group::load(&transaction, group_id)?
            .ok_or(anyhow!("Can't find group with id {:?}", group_id))?;
        let params = group.update_user_key(&transaction)?;
        let owner_domain = conversation.owner_domain();
        let ds_timestamp = self
            .api_clients
            .get(&owner_domain)?
            .ds_update_client(params, group.group_state_ear_key(), group.leaf_signer())
            .await?;
        let group_messages = group.merge_pending_commit(&transaction, None, ds_timestamp)?;

        group.store_update(&transaction)?;

        let conversation_messages =
            Self::store_messages(&mut transaction, conversation_id, group_messages)?;
        transaction.commit()?;
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
        let mut connection = self.sqlite_connection.lock().await;
        let mut transaction = connection.transaction()?;
        let mut conversation =
            Conversation::load(&transaction, &conversation_id)?.ok_or(anyhow!(
                "Can't find conversation with id {}",
                conversation_id.as_uuid()
            ))?;
        let group_id = conversation.group_id();
        // Generate ciphertext
        let mut group = Group::load(&transaction, group_id)?
            .ok_or(anyhow!("Can't find group with id {:?}", group_id))?;
        let past_members = group.members(&transaction);
        // No need to send a message to the server if we are the only member.
        // TODO: Make sure this is what we want.
        let messages = if past_members.len() != 1 {
            let params = group.delete(&transaction)?;
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
            let messages = group.merge_pending_commit(&transaction, None, ds_timestamp)?;
            group.store_update(&transaction)?;
            messages
        } else {
            vec![]
        };

        conversation.set_inactive(&transaction, past_members.into_iter().collect())?;
        let conversation_messages =
            Self::store_messages(&mut transaction, conversation_id, messages)?;
        transaction.commit()?;
        Ok(conversation_messages)
    }

    async fn fetch_messages_from_queue(&self, queue_type: QueueType) -> Result<Vec<QueueMessage>> {
        let connection = &self.sqlite_connection.lock().await;
        let mut remaining_messages = 1;
        let mut messages: Vec<QueueMessage> = Vec::new();
        let mut sequence_number = queue_type.load_sequence_number(connection)?;
        while remaining_messages > 0 {
            let api_client = self.api_clients.default_client()?;
            let mut response = match &queue_type {
                QueueType::As => {
                    api_client
                        .as_dequeue_messages(
                            sequence_number,
                            1_000_000,
                            &self.key_store.signing_key,
                        )
                        .await?
                }
                QueueType::Qs => {
                    api_client
                        .qs_dequeue_messages(
                            &self.qs_client_id,
                            sequence_number,
                            1_000_000,
                            &self.key_store.qs_client_signing_key,
                        )
                        .await?
                }
            };

            remaining_messages = response.remaining_messages_number;
            messages.append(&mut response.messages);

            if let Some(message) = messages.last() {
                sequence_number = message.sequence_number + 1;
                queue_type.update_sequence_number(connection, sequence_number)?;
            }
        }
        Ok(messages)
    }

    pub async fn as_fetch_messages(&self) -> Result<Vec<QueueMessage>> {
        self.fetch_messages_from_queue(QueueType::As).await
    }

    pub async fn qs_fetch_messages(&self) -> Result<Vec<QueueMessage>> {
        self.fetch_messages_from_queue(QueueType::Qs).await
    }

    pub async fn leave_group(&self, conversation_id: ConversationId) -> Result<()> {
        let connection = &self.sqlite_connection.lock().await;
        let conversation = Conversation::load(connection, &conversation_id)?.ok_or(anyhow!(
            "Can't find conversation with id {}",
            conversation_id.as_uuid()
        ))?;
        let group_id = conversation.group_id();
        let mut group = Group::load(connection, group_id)?
            .ok_or(anyhow!("Can't find group with id {:?}", group_id))?;
        let params = group.leave_group(connection)?;
        let owner_domain = conversation.owner_domain();
        self.api_clients
            .get(&owner_domain)?
            .ds_self_remove_client(
                params,
                group.user_auth_key().ok_or(anyhow!("No user auth key"))?,
                group.group_state_ear_key(),
            )
            .await?;
        group.store_update(connection)?;
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
        let mut connection = self.sqlite_connection.lock().await;
        let mut transaction = connection.transaction()?;
        let conversation = Conversation::load(&transaction, &conversation_id)?.ok_or(anyhow!(
            "Can't find conversation with id {}",
            conversation_id.as_uuid()
        ))?;
        let group_id = conversation.group_id();
        let mut group = Group::load(&transaction, group_id)?
            .ok_or(anyhow!("Can't find group with id {:?}", group_id))?;
        let params = group.update(&transaction)?;
        let owner_domain = conversation.owner_domain();
        let ds_timestamp = self
            .api_clients
            .get(&owner_domain)?
            .ds_update_client(params, group.group_state_ear_key(), group.leaf_signer())
            .await?;
        let group_messages = group.merge_pending_commit(&transaction, None, ds_timestamp)?;

        group.store_update(&transaction)?;

        let conversation_messages =
            Self::store_messages(&mut transaction, conversation_id, group_messages)?;
        transaction.commit()?;
        Ok(conversation_messages)
    }

    pub async fn contacts(&self) -> Result<Vec<Contact>, PersistenceError> {
        let connection = &self.sqlite_connection.lock().await;
        let contacts = Contact::load_all(connection)?;
        Ok(contacts)
    }

    pub async fn contact(&self, user_name: &UserName) -> Option<Contact> {
        let connection = &self.sqlite_connection.lock().await;
        Contact::load(connection, user_name).ok().flatten()
    }

    pub async fn partial_contacts(&self) -> Result<Vec<PartialContact>, PersistenceError> {
        let connection = &self.sqlite_connection.lock().await;
        let partial_contact = PartialContact::load_all(connection)?;
        Ok(partial_contact)
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
    pub async fn group_members(
        &self,
        conversation_id: ConversationId,
    ) -> Option<HashSet<UserName>> {
        let connection = &self.sqlite_connection.lock().await;
        let conversation = Conversation::load(connection, &conversation_id).ok()??;

        Group::load(connection, conversation.group_id())
            .ok()?
            .map(|g| g.members(connection))
    }

    pub async fn pending_removes(&self, conversation_id: ConversationId) -> Option<Vec<UserName>> {
        let connection = &self.sqlite_connection.lock().await;
        let conversation = Conversation::load(connection, &conversation_id).ok()??;

        Group::load(connection, conversation.group_id())
            .ok()?
            .map(|group| group.pending_removes(connection))
    }

    pub async fn conversations(&self) -> Result<Vec<Conversation>, PersistenceError> {
        let connection = &self.sqlite_connection.lock().await;
        let conversations = Conversation::load_all(connection)?;
        Ok(conversations)
    }

    pub async fn websocket(&self, timeout: u64, retry_interval: u64) -> Result<QsWebSocket> {
        let api_client = self.api_clients.default_client();
        Ok(api_client?
            .spawn_websocket(self.qs_client_id.clone(), timeout, retry_interval)
            .await?)
    }

    /// Mark all messages in the conversation with the given conversation id and
    /// with a timestamp older than the given timestamp as read.
    pub async fn mark_as_read<
        'b,
        T: 'b + IntoIterator<Item = (&'b ConversationId, &'b TimeStamp)>,
    >(
        &self,
        mark_as_read_data: T,
    ) -> Result<(), PersistenceError> {
        let mut connection = self.sqlite_connection.lock().await;
        let mut transaction = connection.transaction()?;
        Conversation::mark_as_read(&mut transaction, mark_as_read_data)?;
        transaction.commit()?;
        Ok(())
    }

    /// Returns how many messages in the conversation with the given ID are
    /// marked as unread.
    pub async fn unread_message_count(
        &self,
        conversation_id: ConversationId,
    ) -> Result<u32, PersistenceError> {
        let connection = &self.sqlite_connection.lock().await;
        let count = Conversation::unread_message_count(connection, conversation_id)?;
        Ok(count)
    }

    pub fn as_client_id(&self) -> AsClientId {
        self.key_store.signing_key.credential().identity().clone()
    }

    fn store_messages(
        transaction: &mut Transaction,
        conversation_id: ConversationId,
        group_messages: Vec<TimestampedMessage>,
    ) -> Result<Vec<ConversationMessage>> {
        let savepoint = transaction.savepoint()?;
        let mut stored_messages = vec![];
        for timestamped_message in group_messages.into_iter() {
            let message =
                ConversationMessage::from_timestamped_message(conversation_id, timestamped_message);
            message.store(&savepoint)?;
            stored_messages.push(message);
        }
        savepoint.commit()?;
        Ok(stored_messages)
    }

    fn api_clients(&self) -> ApiClients {
        self.api_clients.clone()
    }

    /// Returns the user profile of this [`CoreUser`].
    pub async fn own_user_profile(&self) -> Result<UserProfile, rusqlite::Error> {
        let connection = &self.sqlite_connection.lock().await;
        UserProfile::load(connection, &self.user_name())
            // We unwrap here, because we know that the user exists.
            .map(|user_option| user_option.unwrap())
    }
}
