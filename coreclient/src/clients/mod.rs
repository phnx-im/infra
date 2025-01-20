// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{collections::HashSet, sync::Arc};

use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, Duration, Utc};
use exif::{Reader, Tag};
use opaque_ke::{
    ClientRegistration, ClientRegistrationFinishParameters, ClientRegistrationFinishResult,
    ClientRegistrationStartResult, Identifiers, RegistrationUpload,
};
use openmls::prelude::Ciphersuite;
use own_client_info::OwnClientInfo;
use phnxapiclient::{qs_api::ws::QsWebSocket, ApiClient, ApiClientInitError};
use phnxtypes::{
    codec::PhnxCodec,
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
            EarEncryptable, EarKey, GenericSerializable,
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
        AsClientId, ClientConfig, QsClientId, QsClientReference, QsUserId, QualifiedUserName,
    },
    messages::{
        client_as::{ConnectionPackageTbs, UserConnectionPackagesParams},
        push_token::{EncryptedPushToken, PushToken},
        FriendshipToken, MlsInfraVersion, QueueMessage,
    },
};
use rusqlite::{Connection, Transaction};
use serde::{Deserialize, Serialize};
use store::ClientRecord;
use thiserror::Error;
use tokio_stream::Stream;
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    clients::connection_establishment::{ConnectionEstablishmentPackageTbs, FriendshipPackage},
    contacts::{Contact, ContactAddInfos, PartialContact},
    conversations::{
        messages::{ConversationMessage, TimestampedMessage},
        Conversation, ConversationAttributes,
    },
    key_stores::{queue_ratchets::QueueType, MemoryUserKeyStore},
    store::{StoreNotification, StoreNotifier},
    user_profiles::UserProfile,
    utils::{
        migration::run_migrations,
        persistence::{open_client_db, open_phnx_db},
    },
    ConversationMessageId,
};
use crate::{
    groups::{client_auth_info::StorableClientCredential, Group},
    Asset,
};
use crate::{key_stores::as_credentials::AsCredentials, ConversationId};
use crate::{mimi_content::MimiContent, store::StoreNotificationsSender};
use crate::{
    utils::persistence::{SqliteConnection, Storable},
    Message,
};

use self::{api_clients::ApiClients, create_user::InitialUserState, store::UserCreationState};

pub(crate) mod api_clients;
pub(crate) mod connection_establishment;
pub mod conversations;
mod create_user;
pub(crate) mod own_client_info;
mod persistence;
pub mod process;
pub mod store;
#[cfg(test)]
mod tests;

pub(crate) const CIPHERSUITE: Ciphersuite =
    Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;

pub(crate) const CONNECTION_PACKAGES: usize = 50;
pub(crate) const ADD_PACKAGES: usize = 50;
pub(crate) const CONNECTION_PACKAGE_EXPIRATION: Duration = Duration::days(30);

#[derive(Clone)]
pub struct CoreUser {
    inner: Arc<CoreUserInner>,
}

struct CoreUserInner {
    connection: SqliteConnection,
    api_clients: ApiClients,
    _qs_user_id: QsUserId,
    qs_client_id: QsClientId,
    key_store: MemoryUserKeyStore,
    store_notifications_tx: StoreNotificationsSender,
}

impl CoreUser {
    /// Create a new user with the given `user_name`. If a user with this name
    /// already exists, this will overwrite that user.
    pub async fn new(
        user_name: QualifiedUserName,
        password: &str,
        server_url: impl ToString,
        db_path: &str,
        push_token: Option<PushToken>,
    ) -> Result<Self> {
        let as_client_id = AsClientId::random(user_name)?;
        // Open the phnx db to store the client record
        let phnx_db_connection = open_phnx_db(db_path)?;

        // Open client specific db
        let client_db_connection = open_client_db(&as_client_id, db_path)?;

        Self::new_with_connections(
            as_client_id,
            password,
            server_url,
            push_token,
            SqliteConnection::new(phnx_db_connection),
            SqliteConnection::new(client_db_connection),
        )
        .await
    }

    async fn new_with_connections(
        as_client_id: AsClientId,
        password: &str,
        server_url: impl ToString,
        push_token: Option<PushToken>,
        phnx_db_connection_mutex: SqliteConnection,
        client_db_connection_mutex: SqliteConnection,
    ) -> Result<Self> {
        let server_url = server_url.to_string();
        let api_clients = ApiClients::new(as_client_id.user_name().domain(), server_url.clone());

        let mut client_db_connection = client_db_connection_mutex.lock().await;
        let phnx_db_connection = phnx_db_connection_mutex.lock().await;

        run_migrations(&mut client_db_connection)?;

        let user_creation_state = UserCreationState::new(
            &client_db_connection,
            &phnx_db_connection,
            as_client_id,
            server_url.clone(),
            password,
            push_token,
        )?;

        drop(client_db_connection);
        drop(phnx_db_connection);

        let final_state = user_creation_state
            .complete_user_creation(
                phnx_db_connection_mutex,
                client_db_connection_mutex.clone(),
                &api_clients,
            )
            .await?;

        let client_db_connection = client_db_connection_mutex.lock().await;
        OwnClientInfo {
            server_url,
            qs_user_id: final_state.qs_user_id().clone(),
            qs_client_id: final_state.qs_client_id().clone(),
            as_client_id: final_state.client_id().clone(),
        }
        .store(&client_db_connection)?;
        drop(client_db_connection);

        let self_user = final_state.into_self_user(client_db_connection_mutex, api_clients);

        Ok(self_user)
    }

    /// The same as [`Self::new()`], except that databases are ephemeral and are
    /// dropped together with this instance of CoreUser.
    pub async fn new_ephemeral(
        user_name: impl Into<QualifiedUserName>,
        password: &str,
        server_url: impl ToString,
        push_token: Option<PushToken>,
    ) -> Result<Self> {
        let user_name = user_name.into();
        let as_client_id = AsClientId::random(user_name)?;
        // Open the phnx db to store the client record
        let phnx_db_connection = Connection::open_in_memory()?;

        ClientRecord::create_table(&phnx_db_connection)?;

        // Open client specific db
        let client_db_connection = Connection::open_in_memory()?;

        Self::new_with_connections(
            as_client_id,
            password,
            server_url,
            push_token,
            SqliteConnection::new(phnx_db_connection),
            SqliteConnection::new(client_db_connection),
        )
        .await
    }

    /// Load a user from the database. If a user creation process with a
    /// matching `AsClientId` was interrupted before, this will resume that
    /// process.
    pub async fn load(as_client_id: AsClientId, db_path: &str) -> Result<Option<CoreUser>> {
        let phnx_db_connection = open_phnx_db(db_path)?;

        let mut client_db_connection = open_client_db(&as_client_id, db_path)?;

        run_migrations(&mut client_db_connection)?;

        let Some(user_creation_state) =
            UserCreationState::load(&client_db_connection, &as_client_id)?
        else {
            return Ok(None);
        };

        let api_clients = ApiClients::new(
            as_client_id.user_name().domain(),
            user_creation_state.server_url(),
        );

        let client_db_connection_mutex = SqliteConnection::new(client_db_connection);
        let phnx_db_connection_mutex = SqliteConnection::new(phnx_db_connection);

        let final_state = user_creation_state
            .complete_user_creation(
                phnx_db_connection_mutex,
                client_db_connection_mutex.clone(),
                &api_clients,
            )
            .await?;

        let self_user = final_state.into_self_user(client_db_connection_mutex, api_clients);

        Ok(Some(self_user))
    }

    pub(crate) fn send_store_notification(&self, notification: StoreNotification) {
        if !notification.ops.is_empty() {
            self.inner.store_notifications_tx.notify(notification);
        }
    }

    pub(crate) fn subscribe_to_store_notifications(
        &self,
    ) -> impl Stream<Item = Arc<StoreNotification>> + Send + 'static {
        self.inner.store_notifications_tx.subscribe()
    }

    pub(crate) fn store_notifier(&self) -> StoreNotifier {
        StoreNotifier::new(self.inner.store_notifications_tx.clone())
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
        let connection = &self.inner.connection.lock().await;
        let mut notifier = self.store_notifier();
        user_profile.update(connection, &mut notifier)?;
        notifier.notify();
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
        info!(
            from_bytes = image_bytes.len(),
            to_bytes = buf.len(),
            "Resized profile picture",
        );
        Ok(buf)
    }

    /// Get the user profile of the user with the given [`QualifiedUserName`].
    pub async fn user_profile(&self, user_name: &QualifiedUserName) -> Result<Option<UserProfile>> {
        let connection = &self.inner.connection.lock().await;
        let user = UserProfile::load(connection, user_name)?;
        Ok(user)
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
        invited_users: &[QualifiedUserName],
    ) -> Result<Vec<ConversationMessage>> {
        // Phase 1: Load all the relevant conversation and all the contacts we
        // want to add.
        let connection = self.inner.connection.lock().await;
        let conversation = Conversation::load(&connection, &conversation_id)?.ok_or(anyhow!(
            "Can't find conversation with id {}",
            conversation_id.as_uuid()
        ))?;
        let group_id = conversation.group_id().clone();
        let owner_domain = conversation.owner_domain();

        let mut contact_wai_keys = vec![];
        let mut client_credentials = vec![];
        let mut contacts = vec![];
        for invited_user in invited_users {
            // Get the WAI keys and client credentials for the invited users.
            let contact = Contact::load(&connection, invited_user)?.ok_or(anyhow!(
                "Can't find contact with user name {}",
                invited_user
            ))?;
            contact_wai_keys.push(contact.wai_ear_key().clone());
            let contact_client_credentials = contact
                .clients()
                .iter()
                .filter_map(|client_id| {
                    match StorableClientCredential::load_by_client_id(&connection, client_id) {
                        Ok(Some(client_credential)) => {
                            Some(Ok(ClientCredential::from(client_credential)))
                        }
                        Ok(None) => None,
                        Err(e) => Some(Err(e)),
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            client_credentials.push(contact_client_credentials);
            contacts.push(contact);
        }
        drop(connection);

        // Phase 2: Load add infos for each contact
        // This needs the connection load (and potentially fetch and store).
        let mut contact_add_infos: Vec<ContactAddInfos> = vec![];
        for contact in contacts {
            let add_info = contact
                .fetch_add_infos(
                    self.inner.connection.clone(),
                    self.inner.api_clients.clone(),
                )
                .await?;
            contact_add_infos.push(add_info);
        }

        debug_assert!(contact_add_infos.len() == invited_users.len());

        // Phase 3: Load the group and create the commit to add the new members
        let connection = self.inner.connection.lock().await;
        let mut group = Group::load(&connection, &group_id)?
            .ok_or(anyhow!("Can't find group with id {:?}", group_id))?;
        // Adds new member and staged commit
        let params = group.invite(
            &connection,
            &self.inner.key_store.signing_key,
            contact_add_infos,
            contact_wai_keys,
            client_credentials,
        )?;
        drop(connection);

        // Phase 4: Send the commit to the DS
        // The DS responds with the timestamp of the commit.
        let ds_timestamp = self
            .inner
            .api_clients
            .get(&owner_domain)?
            .ds_add_users(
                params,
                group.group_state_ear_key(),
                group.user_auth_key().ok_or(anyhow!("No user auth key"))?,
            )
            .await?;

        // Phase 5: Merge the commit into the group
        let mut connection = self.inner.connection.lock().await;
        let mut transaction = connection.transaction()?;
        // Now that we know the commit went through, we can merge the commit
        let group_messages = group.merge_pending_commit(&transaction, None, ds_timestamp)?;
        group.store_update(&transaction)?;

        let conversation_messages =
            self.store_messages(&mut transaction, conversation_id, group_messages)?;
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
        target_users: &[QualifiedUserName],
    ) -> Result<Vec<ConversationMessage>> {
        // Phase 1: Load the group and conversation and prepare the commit.
        let connection = self.inner.connection.lock().await;
        let conversation = Conversation::load(&connection, &conversation_id)?.ok_or(anyhow!(
            "Can't find conversation with id {}",
            conversation_id.as_uuid()
        ))?;
        let group_id = conversation.group_id();
        let mut group = Group::load(&connection, group_id)?
            .ok_or(anyhow!("Can't find group with id {:?}", group_id))?;
        let clients = target_users
            .iter()
            .flat_map(|user_name| group.user_client_ids(&connection, user_name))
            .collect::<Vec<_>>();
        let params = group.remove(&connection, clients)?;
        drop(connection);

        // Phase 2: Send the commit to the DS
        let ds_timestamp = self
            .inner
            .api_clients
            .get(&conversation.owner_domain())?
            .ds_remove_users(
                params,
                group.group_state_ear_key(),
                group.user_auth_key().ok_or(anyhow!("No user auth key"))?,
            )
            .await?;

        // Phase 3: Merge the commit into the group
        let mut connection = self.inner.connection.lock().await;
        let mut transaction = connection.transaction()?;
        let group_messages = group.merge_pending_commit(&transaction, None, ds_timestamp)?;
        group.store_update(&transaction)?;

        let conversation_messages =
            self.store_messages(&mut transaction, conversation_id, group_messages)?;
        transaction.commit()?;
        drop(connection);

        Ok(conversation_messages)
    }

    /// Send a message and return it. Note that the message has already been
    /// sent to the DS and has internally been stored in the conversation store.
    pub async fn send_message(
        &self,
        conversation_id: ConversationId,
        content: MimiContent,
    ) -> Result<ConversationMessage> {
        // Phase 1: Load the conversation and group
        let (group, params, conversation, mut conversation_message) = {
            let mut notifier = self.store_notifier();
            let mut connection = self.inner.connection.lock().await;
            let mut transaction = connection.transaction()?;
            let conversation =
                Conversation::load(&transaction, &conversation_id)?.ok_or(anyhow!(
                    "Can't find conversation with id {}",
                    conversation_id.as_uuid()
                ))?;
            let group_id = conversation.group_id();
            // Store the message as unsent so that we don't lose it in case
            // something goes wrong.
            let conversation_message = ConversationMessage::new_unsent_message(
                self.user_name().to_string(),
                conversation_id,
                content.clone(),
            );
            conversation_message.store(&transaction, &mut notifier)?;

            // Notify as early as possible to react to the not yet sent message
            notifier.notify();

            let mut group = Group::load(&transaction, group_id)?
                .ok_or(anyhow!("Can't find group with id {group_id:?}"))?;
            let params = group.create_message(&transaction, content)?;
            // Immediately write the group back. No need to wait for the DS to
            // confirm as this is just an application message.
            group.store_update(&transaction)?;
            // Also, mark the message (and all messages preceeding it) as read.
            let mut notifier = self.store_notifier();
            Conversation::mark_as_read(
                &mut transaction,
                &mut notifier,
                vec![(conversation.id(), conversation_message.timestamp())].into_iter(),
            )?;
            transaction.commit()?;
            notifier.notify();
            drop(connection);
            (group, params, conversation, conversation_message)
        };

        // Phase 2: Send message to DS
        let ds_timestamp = self
            .inner
            .api_clients
            .get(&conversation.owner_domain())?
            .ds_send_message(params, group.leaf_signer(), group.group_state_ear_key())
            .await?;

        // Phase 3: Mark the message as sent and read (again).
        let mut connection = self.inner.connection.lock().await;
        let mut notifier = self.store_notifier();
        conversation_message.mark_as_sent(&connection, &mut notifier, ds_timestamp)?;
        let mut transaction = connection.transaction()?;
        Conversation::mark_as_read(
            &mut transaction,
            &mut notifier,
            vec![(conversation.id(), conversation_message.timestamp())].into_iter(),
        )?;
        transaction.commit()?;
        notifier.notify();

        Ok(conversation_message)
    }

    /// Re-try sending a message, where sending previously failed.
    pub async fn re_send_message(&self, local_message_id: Uuid) -> Result<()> {
        // Phase 1: Load the unsent message
        let connection = self.inner.connection.lock().await;
        let mut unsent_message = ConversationMessage::load(&connection, &local_message_id)?.ok_or(
            anyhow!("Can't find unsent message with id {}", local_message_id),
        )?;
        let content = match unsent_message.message() {
            Message::Content(content_message) if !content_message.was_sent() => {
                content_message.content().clone()
            }
            _ => bail!("Message with id {} was already sent", local_message_id),
        };
        let conversation_id = unsent_message.conversation_id();
        let conversation = Conversation::load(&connection, &conversation_id)?.ok_or(anyhow!(
            "Can't find conversation with id {}",
            conversation_id.as_uuid()
        ))?;
        let group_id = conversation.group_id();
        let mut group = Group::load(&connection, group_id)?
            .ok_or(anyhow!("Can't find group with id {:?}", group_id))?;
        let params = group.create_message(&connection, content)?;
        drop(connection);

        // Phase 2: Send message to DS
        let ds_timestamp = self
            .inner
            .api_clients
            .get(&conversation.owner_domain())?
            .ds_send_message(params, group.leaf_signer(), group.group_state_ear_key())
            .await?;

        // Phase 3: Merge the commit into the group & update conversation
        let mut connection = self.inner.connection.lock().await;
        let mut notifier = self.store_notifier();
        unsent_message.mark_as_sent(&connection, &mut notifier, ds_timestamp)?;
        group.store_update(&connection)?;
        let mut transaction = connection.transaction()?;
        Conversation::mark_as_read(
            &mut transaction,
            &mut notifier,
            vec![(conversation.id(), unsent_message.timestamp())].into_iter(),
        )?;
        transaction.commit()?;
        notifier.notify();

        Ok(())
    }

    /// Create a connection with a new user.
    ///
    /// Returns the [`ConversationId`] of the newly created connection
    /// conversation.
    pub async fn add_contact(&self, user_name: QualifiedUserName) -> Result<ConversationId> {
        let params = UserConnectionPackagesParams {
            user_name: user_name.clone(),
        };
        // Phase 1: Fetch connection key packages from the AS
        let user_domain = user_name.domain();
        info!(%user_name, "Adding contact");
        let user_key_packages = self
            .inner
            .api_clients
            .get(&user_domain)?
            .as_user_connection_packages(params)
            .await?;

        // The AS should return an error if the user does not exist, but we
        // check here locally just to be sure.
        if user_key_packages.connection_packages.is_empty() {
            return Err(anyhow!("User {} does not exist", user_name));
        }
        // Phase 2: Verify the connection key packages
        info!("Verifying connection packages");
        let mut verified_connection_packages = vec![];
        for connection_package in user_key_packages.connection_packages.into_iter() {
            let as_intermediate_credential = AsCredentials::get(
                self.inner.connection.clone(),
                &self.inner.api_clients,
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

        // Phase 3: Request a group id from the DS
        info!("Requesting group id");
        let group_id = self
            .inner
            .api_clients
            .default_client()?
            .ds_request_group_id()
            .await?;

        // Phase 4: Prepare the connection locally
        info!("Creating local connection group");
        let title = format!("Connection group: {} - {}", self.user_name(), user_name);
        let conversation_attributes = ConversationAttributes::new(title.to_string(), None);
        let group_data = PhnxCodec::to_vec(&conversation_attributes)?.into();
        let mut connection = self.inner.connection.lock().await;
        let (connection_group, partial_params) = Group::create_group(
            &mut connection,
            &self.inner.key_store.signing_key,
            group_id.clone(),
            group_data,
        )?;
        connection_group.store(&connection)?;

        // TODO: Once we allow multi-client, invite all our other clients to the
        // connection group.

        let own_user_profile = UserProfile::load(&connection, &self.user_name())
            // We unwrap here, because we know that the user exists.
            .map(|user_option| user_option.unwrap())?;

        // Create the connection conversation
        let conversation = Conversation::new_connection_conversation(
            group_id.clone(),
            user_name.clone(),
            conversation_attributes,
        )?;
        let mut notifier = self.store_notifier();
        conversation.store(&connection, &mut notifier)?;

        let friendship_package = FriendshipPackage {
            friendship_token: self.inner.key_store.friendship_token.clone(),
            add_package_ear_key: self.inner.key_store.add_package_ear_key.clone(),
            client_credential_ear_key: self.inner.key_store.client_credential_ear_key.clone(),
            signature_ear_key_wrapper_key: self
                .inner
                .key_store
                .signature_ear_key_wrapper_key
                .clone(),
            wai_ear_key: self.inner.key_store.wai_ear_key.clone(),
            user_profile: own_user_profile,
        };

        let friendship_package_ear_key = FriendshipPackageEarKey::random()?;

        // Create and persist a new partial contact
        PartialContact::new(
            user_name.clone(),
            conversation.id(),
            friendship_package_ear_key.clone(),
        )
        .store(&connection, &mut notifier)?;

        // Store the user profile of the partial contact (we don't have a
        // display name or a profile picture yet)
        UserProfile::new(user_name, None, None).store(&connection, &mut notifier)?;

        drop(connection);

        // Create a connection establishment package
        let connection_establishment_package = ConnectionEstablishmentPackageTbs {
            sender_client_credential: self.inner.key_store.signing_key.credential().clone(),
            connection_group_id: group_id,
            connection_group_ear_key: connection_group.group_state_ear_key().clone(),
            connection_group_credential_key: connection_group.credential_ear_key().clone(),
            connection_group_signature_ear_key_wrapper_key: connection_group
                .signature_ear_key_wrapper_key()
                .clone(),
            friendship_package_ear_key,
            friendship_package,
        }
        .sign(&self.inner.key_store.signing_key)?;

        let client_reference = self.create_own_client_reference();
        let encrypted_client_credential = self
            .inner
            .key_store
            .signing_key
            .credential()
            .encrypt(connection_group.credential_ear_key())?;
        let params = partial_params.into_params(encrypted_client_credential, client_reference);

        // Phase 5: Create the connection group on the DS and send off the
        // connection establishment packages
        info!("Creating connection group on DS");
        self.inner
            .api_clients
            .default_client()?
            .ds_create_group(
                params,
                connection_group.group_state_ear_key(),
                connection_group
                    .user_auth_key()
                    .ok_or(anyhow!("No user auth key"))?,
            )
            .await?;

        // Encrypt the connection establishment package for each connection and send it off.
        for connection_package in verified_connection_packages {
            let ciphertext = connection_establishment_package.encrypt(
                connection_package.encryption_key(),
                &[],
                &[],
            );
            let client_id = connection_package.client_credential().identity();

            self.inner
                .api_clients
                .get(&user_domain)?
                .as_enqueue_message(client_id, ciphertext)
                .await?;
        }

        notifier.notify();

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
        conversation_id: &ConversationId,
    ) -> Result<Vec<ConversationMessage>> {
        // Phase 1: Load the conversation and the group
        let connection = self.inner.connection.lock().await;
        let conversation = Conversation::load(&connection, conversation_id)?.ok_or(anyhow!(
            "Can't find conversation with id {}",
            conversation_id.as_uuid()
        ))?;
        let group_id = conversation.group_id();
        // Generate ciphertext
        let mut group = Group::load(&connection, group_id)?
            .ok_or(anyhow!("Can't find group with id {:?}", group_id))?;
        let params = group.update_user_key(&connection)?;
        drop(connection);

        let owner_domain = conversation.owner_domain();

        // Phase 2: Send the update to the DS
        let ds_timestamp = self
            .inner
            .api_clients
            .get(&owner_domain)?
            .ds_update_client(params, group.group_state_ear_key(), group.leaf_signer())
            .await?;

        // Phase 3: Store the updated group
        let mut connection = self.inner.connection.lock().await;
        let mut transaction = connection.transaction()?;

        let group_messages = group.merge_pending_commit(&transaction, None, ds_timestamp)?;

        group.store_update(&transaction)?;

        let conversation_messages =
            self.store_messages(&mut transaction, *conversation_id, group_messages)?;
        transaction.commit()?;
        drop(connection);

        Ok(conversation_messages)
    }

    /// Delete the conversation with the given [`ConversationId`].
    ///
    /// Since this function causes the creation of an MLS commit, it can cause
    /// more than one effect on the group. As a result this function returns a
    /// vector of [`ConversationMessage`]s that represents the changes to the
    /// group. Note that these returned message have already been persisted.
    pub async fn delete_conversation(
        &self,
        conversation_id: ConversationId,
    ) -> Result<Vec<ConversationMessage>> {
        // Phase 1: Load the conversation and the group
        let connection = self.inner.connection.lock().await;
        let mut conversation =
            Conversation::load(&connection, &conversation_id)?.ok_or(anyhow!(
                "Can't find conversation with id {}",
                conversation_id.as_uuid()
            ))?;
        let group_id = conversation.group_id();
        // Generate ciphertext
        let mut group = Group::load(&connection, group_id)?
            .ok_or(anyhow!("Can't find group with id {:?}", group_id))?;
        let past_members = group.members(&connection);
        drop(connection);

        // No need to send a message to the server if we are the only member.
        // TODO: Make sure this is what we want.
        let messages = if past_members.len() != 1 {
            // Phase 2: Create the delete commit
            let connection = self.inner.connection.lock().await;
            let params = group.delete(&connection)?;
            drop(connection);

            let owner_domain = conversation.owner_domain();
            // Phase 3: Send the delete to the DS
            let ds_timestamp = self
                .inner
                .api_clients
                .get(&owner_domain)?
                .ds_delete_group(
                    params,
                    group.user_auth_key().ok_or(anyhow!("No user auth key"))?,
                    group.group_state_ear_key(),
                )
                .await?;

            // Phase 4: Merge the commit into the group
            let connection = self.inner.connection.lock().await;
            let messages = group.merge_pending_commit(&connection, None, ds_timestamp)?;
            group.store_update(&connection)?;
            drop(connection);
            messages
        } else {
            vec![]
        };

        // Phase 4: Set the conversation to inactive
        let mut connection = self.inner.connection.lock().await;
        let mut transaction = connection.transaction()?;
        let mut notifier = self.store_notifier();
        conversation.set_inactive(
            &transaction,
            &mut notifier,
            past_members.into_iter().collect(),
        )?;
        let conversation_messages =
            self.store_messages(&mut transaction, conversation_id, messages)?;
        transaction.commit()?;
        drop(connection);

        notifier.notify();

        Ok(conversation_messages)
    }

    async fn fetch_messages_from_queue(&self, queue_type: QueueType) -> Result<Vec<QueueMessage>> {
        let connection = self.inner.connection.lock().await;
        let mut remaining_messages = 1;
        let mut messages: Vec<QueueMessage> = Vec::new();
        let mut sequence_number = queue_type.load_sequence_number(&connection)?;
        drop(connection);

        while remaining_messages > 0 {
            let api_client = self.inner.api_clients.default_client()?;
            let mut response = match &queue_type {
                QueueType::As => {
                    api_client
                        .as_dequeue_messages(
                            sequence_number,
                            1_000_000,
                            &self.inner.key_store.signing_key,
                        )
                        .await?
                }
                QueueType::Qs => {
                    api_client
                        .qs_dequeue_messages(
                            &self.inner.qs_client_id,
                            sequence_number,
                            1_000_000,
                            &self.inner.key_store.qs_client_signing_key,
                        )
                        .await?
                }
            };

            remaining_messages = response.remaining_messages_number;
            messages.append(&mut response.messages);

            let connection = self.inner.connection.lock().await;
            if let Some(message) = messages.last() {
                sequence_number = message.sequence_number + 1;
                queue_type.update_sequence_number(&connection, sequence_number)?;
            }
            drop(connection);
        }
        Ok(messages)
    }

    pub async fn as_fetch_messages(&self) -> Result<Vec<QueueMessage>> {
        self.fetch_messages_from_queue(QueueType::As).await
    }

    pub async fn qs_fetch_messages(&self) -> Result<Vec<QueueMessage>> {
        self.fetch_messages_from_queue(QueueType::Qs).await
    }

    pub async fn leave_conversation(&self, conversation_id: ConversationId) -> Result<()> {
        // Phase 1: Load the conversation and the group
        let connection = self.inner.connection.lock().await;
        let conversation = Conversation::load(&connection, &conversation_id)?.ok_or(anyhow!(
            "Can't find conversation with id {}",
            conversation_id.as_uuid()
        ))?;
        let group_id = conversation.group_id();
        let mut group = Group::load(&connection, group_id)?
            .ok_or(anyhow!("Can't find group with id {:?}", group_id))?;

        let params = group.leave_group(&connection)?;
        drop(connection);

        let owner_domain = conversation.owner_domain();

        // Phase 2: Send the leave to the DS
        self.inner
            .api_clients
            .get(&owner_domain)?
            .ds_self_remove_client(
                params,
                group.user_auth_key().ok_or(anyhow!("No user auth key"))?,
                group.group_state_ear_key(),
            )
            .await?;

        // Phase 3: Merge the commit into the group
        let connection = self.inner.connection.lock().await;
        group.store_update(&connection)?;
        drop(connection);

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
        &self,
        conversation_id: ConversationId,
    ) -> Result<Vec<ConversationMessage>> {
        // Phase 1: Load the conversation and the group
        let connection = self.inner.connection.lock().await;
        let conversation = Conversation::load(&connection, &conversation_id)?.ok_or(anyhow!(
            "Can't find conversation with id {}",
            conversation_id.as_uuid()
        ))?;
        let group_id = conversation.group_id();
        let mut group = Group::load(&connection, group_id)?
            .ok_or(anyhow!("Can't find group with id {:?}", group_id))?;
        let params = group.update(&connection)?;
        drop(connection);

        let owner_domain = conversation.owner_domain();

        // Phase 2: Send the update to the DS
        let ds_timestamp = self
            .inner
            .api_clients
            .get(&owner_domain)?
            .ds_update_client(params, group.group_state_ear_key(), group.leaf_signer())
            .await?;

        // Phase 3: Merge the commit into the group
        let mut connection = self.inner.connection.lock().await;
        let mut transaction = connection.transaction()?;

        let group_messages = group.merge_pending_commit(&transaction, None, ds_timestamp)?;

        group.store_update(&transaction)?;

        let conversation_messages =
            self.store_messages(&mut transaction, conversation_id, group_messages)?;
        transaction.commit()?;
        drop(connection);

        Ok(conversation_messages)
    }

    pub async fn contacts(&self) -> Result<Vec<Contact>, rusqlite::Error> {
        let connection = &self.inner.connection.lock().await;
        let contacts = Contact::load_all(connection)?;
        Ok(contacts)
    }

    pub async fn contact(&self, user_name: &QualifiedUserName) -> Option<Contact> {
        let connection = &self.inner.connection.lock().await;
        Contact::load(connection, user_name).ok().flatten()
    }

    pub async fn try_contact(
        &self,
        user_name: &QualifiedUserName,
    ) -> rusqlite::Result<Option<Contact>> {
        let connection = &self.inner.connection.lock().await;
        Contact::load(connection, user_name)
    }

    pub async fn partial_contacts(&self) -> Result<Vec<PartialContact>, rusqlite::Error> {
        let connection = &self.inner.connection.lock().await;
        let partial_contact = PartialContact::load_all(connection)?;
        Ok(partial_contact)
    }

    fn create_own_client_reference(&self) -> QsClientReference {
        let sealed_reference = ClientConfig {
            client_id: self.inner.qs_client_id.clone(),
            push_token_ear_key: Some(self.inner.key_store.push_token_ear_key.clone()),
        }
        .encrypt(&self.inner.key_store.qs_client_id_encryption_key, &[], &[]);
        QsClientReference {
            client_homeserver_domain: self.user_name().domain(),
            sealed_reference,
        }
    }

    pub fn user_name(&self) -> QualifiedUserName {
        self.inner
            .key_store
            .signing_key
            .credential()
            .identity()
            .user_name()
    }

    /// Returns None if there is no conversation with the given id.
    pub async fn conversation_participants(
        &self,
        conversation_id: ConversationId,
    ) -> Option<HashSet<QualifiedUserName>> {
        let connection = &self.inner.connection.lock().await;
        let conversation = Conversation::load(connection, &conversation_id).ok()??;

        Group::load(connection, conversation.group_id())
            .ok()?
            .map(|g| g.members(connection))
    }

    pub(crate) async fn try_conversation_participants(
        &self,
        conversation_id: ConversationId,
    ) -> Result<Option<HashSet<QualifiedUserName>>> {
        let connection = &self.inner.connection.lock().await;
        let Some(conversation) = Conversation::load(connection, &conversation_id)? else {
            return Ok(None);
        };
        let Some(group) = Group::load(connection, conversation.group_id())? else {
            return Ok(None);
        };
        Ok(Some(group.members(connection)))
    }

    pub async fn pending_removes(
        &self,
        conversation_id: ConversationId,
    ) -> Option<Vec<QualifiedUserName>> {
        let connection = &self.inner.connection.lock().await;
        let conversation = Conversation::load(connection, &conversation_id).ok()??;

        Group::load(connection, conversation.group_id())
            .ok()?
            .map(|group| group.pending_removes(connection))
    }

    pub async fn websocket(&self, timeout: u64, retry_interval: u64) -> Result<QsWebSocket> {
        let api_client = self.inner.api_clients.default_client();
        Ok(api_client?
            .spawn_websocket(self.inner.qs_client_id.clone(), timeout, retry_interval)
            .await?)
    }

    /// Mark all messages in the conversation with the given conversation id and
    /// with a timestamp older than the given timestamp as read.
    pub async fn mark_as_read<T: IntoIterator<Item = (ConversationId, DateTime<Utc>)>>(
        &self,
        mark_as_read_data: T,
    ) -> Result<(), rusqlite::Error> {
        let mut connection = self.inner.connection.lock().await;
        let mut transaction = connection.transaction()?;
        let mut notifier = self.store_notifier();
        Conversation::mark_as_read(&mut transaction, &mut notifier, mark_as_read_data)?;
        transaction.commit()?;
        notifier.notify();
        Ok(())
    }

    /// Mark all messages in the conversation with the given conversation id and
    /// with a timestamp older than the given timestamp as read.
    pub async fn mark_conversation_as_read(
        &self,
        conversation_id: ConversationId,
        until: ConversationMessageId,
    ) -> Result<bool, rusqlite::Error> {
        let connection = self.inner.connection.lock().await;
        let mut notifier = self.store_notifier();
        let marked_as_read = Conversation::mark_as_read_until_message_id(
            &connection,
            &mut notifier,
            conversation_id,
            until,
        )?;
        notifier.notify();
        Ok(marked_as_read)
    }

    /// Returns how many messages are marked as unread across all conversations.
    pub async fn global_unread_messages_count(&self) -> Result<u32, rusqlite::Error> {
        let connection = &self.inner.connection.lock().await;
        let count = Conversation::global_unread_message_count(connection)?;
        Ok(count)
    }

    /// Returns how many messages in the conversation with the given ID are
    /// marked as unread.
    pub async fn unread_messages_count(&self, conversation_id: ConversationId) -> u32 {
        let connection = &self.inner.connection.lock().await;
        Conversation::unread_messages_count(connection, conversation_id).unwrap_or_else(|error| {
            error!(%error, "Error while fetching unread messages count");
            0
        })
    }

    pub(crate) async fn try_messages_count(
        &self,
        conversation_id: ConversationId,
    ) -> Result<usize, rusqlite::Error> {
        let connection = &self.inner.connection.lock().await;
        let count = Conversation::messages_count(connection, conversation_id)?;
        Ok(usize::try_from(count).expect("usize overflow"))
    }

    pub(crate) async fn try_unread_messages_count(
        &self,
        conversation_id: ConversationId,
    ) -> Result<usize, rusqlite::Error> {
        let connection = &self.inner.connection.lock().await;
        let count = Conversation::unread_messages_count(connection, conversation_id)?;
        Ok(usize::try_from(count).expect("usize overflow"))
    }

    /// Updates the client's push token on the QS.
    pub async fn update_push_token(&self, push_token: Option<PushToken>) -> Result<()> {
        let client_id = self.inner.qs_client_id.clone();
        // Ratchet encryption key
        let queue_encryption_key = self
            .inner
            .key_store
            .qs_queue_decryption_key
            .encryption_key();
        // Signung key
        let signing_key = self.inner.key_store.qs_client_signing_key.clone();

        // Encrypt the push token, if there is one.
        let encrypted_push_token = match push_token {
            Some(push_token) => {
                let encrypted_push_token = EncryptedPushToken::from(
                    self.inner
                        .key_store
                        .push_token_ear_key
                        .encrypt(&GenericSerializable::serialize(&push_token)?)?,
                );
                Some(encrypted_push_token)
            }
            None => None,
        };

        self.inner
            .api_clients
            .default_client()?
            .qs_update_client(
                client_id,
                queue_encryption_key,
                encrypted_push_token,
                &signing_key,
            )
            .await?;
        Ok(())
    }

    pub fn as_client_id(&self) -> AsClientId {
        self.inner
            .key_store
            .signing_key
            .credential()
            .identity()
            .clone()
    }

    fn store_messages(
        &self,
        transaction: &mut Transaction,
        conversation_id: ConversationId,
        group_messages: Vec<TimestampedMessage>,
    ) -> Result<Vec<ConversationMessage>> {
        let mut notifier = self.store_notifier();
        let savepoint = transaction.savepoint()?;
        let mut stored_messages = vec![];
        for timestamped_message in group_messages.into_iter() {
            let message =
                ConversationMessage::from_timestamped_message(conversation_id, timestamped_message);
            message.store(&savepoint, &mut notifier)?;
            stored_messages.push(message);
        }
        savepoint.commit()?;
        notifier.notify();
        Ok(stored_messages)
    }

    /// Returns the user profile of this [`CoreUser`].
    pub async fn own_user_profile(&self) -> Result<UserProfile, rusqlite::Error> {
        let connection = &self.inner.connection.lock().await;
        UserProfile::load(connection, &self.user_name())
            // We unwrap here, because we know that the user exists.
            .map(|user_option| user_option.unwrap())
    }
}
