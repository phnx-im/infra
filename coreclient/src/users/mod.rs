// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::{anyhow, Result};
use opaque_ke::{
    ClientRegistration, ClientRegistrationFinishParameters, ClientRegistrationFinishResult,
    ClientRegistrationStartResult, Identifiers,
};
use phnxapiclient::{qs_api::ws::QsWebSocket, ApiClient};
use phnxbackend::{
    auth_service::{
        credentials::{
            keys::{ClientSigningKey, InfraCredentialSigningKey},
            AsCredential, AsIntermediateCredential, ClientCredential, ClientCredentialCsr,
            ClientCredentialPayload, CredentialFingerprint, ExpirationData,
        },
        AsClientId, OpaqueRegistrationRecord, OpaqueRegistrationRequest, UserName,
    },
    crypto::{
        ear::{
            keys::{
                AddPackageEarKey, ClientCredentialEarKey, FriendshipPackageEarKey, PushTokenEarKey,
                SignatureEarKey, SignatureEarKeyWrapperKey, WelcomeAttributionInfoEarKey,
            },
            EarDecryptable, EarEncryptable,
        },
        hpke::HpkeEncryptable,
        kdf::keys::RatchetSecret,
        signatures::{
            keys::{QsClientSigningKey, QsUserSigningKey},
            signable::{Signable, Verifiable},
        },
        ConnectionDecryptionKey, OpaqueCiphersuite, RatchetDecryptionKey,
    },
    ds::{api::QS_CLIENT_REFERENCE_EXTENSION_TYPE, group_state::EncryptedClientCredential},
    messages::{
        client_as::{
            AsQueueRatchet, ConnectionEstablishmentPackageTbs, ConnectionPackageTbs,
            FriendshipPackage, UserConnectionPackagesParams,
        },
        client_ds::QsQueueRatchet,
        client_ds_out::CreateGroupParamsOut,
        FriendshipToken, MlsInfraVersion, QueueMessage,
    },
    qs::{
        AddPackage, ClientConfig, ClientIdEncryptionKey, Fqdn, QsClientId, QsClientReference,
        QsUserId, QsVerifyingKey,
    },
};
use rand::rngs::OsRng;

use crate::{
    contacts::{Contact, ContactAddInfos, PartialContact},
    users::key_store::AsCredentials,
};

use self::key_store::MemoryUserKeyStore;

use super::*;

pub mod key_store;
pub mod process;

pub(crate) const CIPHERSUITE: Ciphersuite =
    Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;

pub(crate) const CONNECTION_PACKAGES: usize = 50;
pub(crate) const ADD_PACKAGES: usize = 50;
pub(crate) const CONNECTION_PACKAGE_EXPIRATION_DAYS: i64 = 30;

pub(crate) struct ApiClients {
    // We store our own domain such that we can manually map our own domain to
    // an API client that uses an IP address instead of the actual domain. This
    // is a temporary workaround and should probably be replaced by a more
    // thought-out mechanism.
    own_domain: Fqdn,
    own_hostname: String,
    clients: HashMap<String, ApiClient>,
}

impl ApiClients {
    fn new(own_domain: Fqdn, own_server_url: impl ToString) -> Self {
        let own_server_url = own_server_url.to_string();
        Self {
            own_domain,
            own_hostname: own_server_url,
            clients: HashMap::new(),
        }
    }

    pub(crate) fn get(&mut self, domain: &Fqdn) -> Result<&ApiClient> {
        let lookup_domain = if domain == &self.own_domain {
            self.own_hostname.clone()
        } else {
            domain.clone().to_string()
        };
        let client = self
            .clients
            .entry(lookup_domain.clone())
            .or_insert(ApiClient::initialize(lookup_domain)?);
        Ok(client)
    }

    fn default_client(&mut self) -> Result<&ApiClient> {
        let own_domain = self.own_domain.clone();
        self.get(&own_domain)
    }
}

pub struct SelfUser<T: Notifiable> {
    pub(crate) crypto_backend: OpenMlsRustCrypto,
    pub(crate) notification_hub: NotificationHub<T>,
    api_clients: ApiClients,
    pub(crate) user_name: UserName,
    pub(crate) qs_user_id: QsUserId,
    pub(crate) qs_client_id: QsClientId,
    pub(crate) qs_client_sequence_number_start: u64,
    pub(crate) as_client_sequence_number_start: u64,
    pub(crate) conversation_store: ConversationStore,
    pub(crate) group_store: GroupStore,
    pub(crate) key_store: MemoryUserKeyStore,
    pub(crate) contacts: HashMap<UserName, Contact>,
    pub(crate) partial_contacts: HashMap<UserName, PartialContact>,
}

impl<T: Notifiable> SelfUser<T> {
    /// Create a new user with the given name and a fresh set of credentials.
    pub async fn new(
        user_name: impl Into<UserName>,
        password: &str,
        server_url: impl ToString,
        notification_hub: NotificationHub<T>,
    ) -> Result<Self> {
        let user_name = user_name.into();
        log::debug!("Creating new user {}", user_name);
        let crypto_backend = OpenMlsRustCrypto::default();
        // Let's turn TLS off for now.
        let domain = user_name.domain();
        let mut api_clients = ApiClients::new(user_name.domain(), server_url);

        let as_credentials = AsCredentials::new(&mut api_clients, &domain).await?;

        let api_client = api_clients.default_client()?;

        let as_client_id = AsClientId::random(crypto_backend.rand(), user_name.clone())?;
        let (client_credential_csr, prelim_signing_key) =
            ClientCredentialCsr::new(as_client_id, SignatureScheme::ED25519)?;

        let as_credentials_response = api_client.as_as_credentials().await?;
        let as_intermediate_credentials: Vec<AsIntermediateCredential> = as_credentials_response
            .as_intermediate_credentials
            .into_iter()
            .map(|as_inter_cred| {
                let as_credential = as_credentials_response
                    .as_credentials
                    .iter()
                    .find(|as_cred| {
                        if let Ok(fingerprint) = as_cred.fingerprint() {
                            &fingerprint == as_inter_cred.signer_fingerprint()
                        } else {
                            false
                        }
                    })
                    .ok_or(anyhow!(
                        "Can't find AS credential with matching fingerprint"
                    ))?;
                let credential: AsIntermediateCredential =
                    as_inter_cred.verify(as_credential.verifying_key())?;
                Ok(credential)
            })
            .collect::<Result<Vec<_>>>()?;

        let chosen_inter_credential = as_intermediate_credentials
            .first()
            .ok_or(anyhow!("AS didn't return any intermediate credentials"))?;

        let client_credential_payload = ClientCredentialPayload::new(
            client_credential_csr,
            None,
            chosen_inter_credential.fingerprint()?,
        );

        // Let's do OPAQUE registration.
        // First get the server setup information.
        let mut client_rng = OsRng;
        let client_registration_start_result: ClientRegistrationStartResult<OpaqueCiphersuite> =
            ClientRegistration::<OpaqueCiphersuite>::start(&mut client_rng, password.as_bytes())
                .map_err(|e| anyhow!("Error starting OPAQUE handshake: {:?}", e))?;

        let opaque_registration_request = OpaqueRegistrationRequest {
            client_message: client_registration_start_result.message,
        };

        // Register the user with the backend.
        let response = api_client
            .as_initiate_create_user(client_credential_payload, opaque_registration_request)
            .await?;

        // Complete the OPAQUE registration.
        let user_name_bytes = user_name.to_bytes();
        let identifiers = Identifiers {
            client: Some(&user_name_bytes),
            server: Some(domain.as_bytes()),
        };
        let response_parameters = ClientRegistrationFinishParameters::new(identifiers, None);
        let client_registration_finish_result: ClientRegistrationFinishResult<OpaqueCiphersuite> =
            client_registration_start_result
                .state
                .finish(
                    &mut client_rng,
                    password.as_bytes(),
                    response.opaque_registration_response.server_message,
                    response_parameters,
                )
                .map_err(|e| anyhow!("Error finishing OPAQUE handshake: {:?}", e))?;

        let credential: ClientCredential = response
            .client_credential
            .verify(chosen_inter_credential.verifying_key())?;

        let signing_key = ClientSigningKey::from_prelim_key(prelim_signing_key, credential)?;
        let as_queue_decryption_key = RatchetDecryptionKey::generate()?;
        let as_initial_ratchet_secret = RatchetSecret::random()?;
        let qs_queue_decryption_key = RatchetDecryptionKey::generate()?;
        let qs_initial_ratchet_secret = RatchetSecret::random()?;
        let qs_client_signing_key = QsClientSigningKey::random()?;
        let qs_user_signing_key = QsUserSigningKey::random()?;

        let leaf_signers = HashMap::new();
        // TODO: The following five keys should be derived from a single
        // friendship key. Once that's done, remove the random constructors.
        let friendship_token = FriendshipToken::random()?;
        let add_package_ear_key = AddPackageEarKey::random()?;
        let client_credential_ear_key = ClientCredentialEarKey::random()?;
        let signature_ear_key_wrapper_key = SignatureEarKeyWrapperKey::random()?;
        let wai_ear_key: WelcomeAttributionInfoEarKey = WelcomeAttributionInfoEarKey::random()?;
        let push_token_ear_key = PushTokenEarKey::random()?;
        let qs_verifying_key = api_client.qs_verifying_key().await?.verifying_key;
        let qs_encryption_key = api_client.qs_encryption_key().await?.encryption_key;
        let connection_decryption_key = ConnectionDecryptionKey::generate()?;

        let key_store = MemoryUserKeyStore {
            signing_key,
            as_queue_decryption_key,
            as_queue_ratchet: as_initial_ratchet_secret.clone().try_into()?,
            connection_decryption_key,
            as_credentials,
            qs_client_signing_key,
            qs_user_signing_key,
            qs_queue_decryption_key,
            qs_queue_ratchet: qs_initial_ratchet_secret.clone().try_into()?,
            qs_verifying_keys: [(user_name.domain(), qs_verifying_key)].into(),
            push_token_ear_key,
            friendship_token,
            add_package_ear_key,
            client_credential_ear_key,
            signature_ear_key_wrapper_key,
            wai_ear_key,
            leaf_signers,
            qs_client_id_encryption_key: qs_encryption_key,
        };

        // TODO: For now, we use the same ConnectionDecryptionKey for all
        // connection packages.

        let mut connection_packages = vec![];
        for _ in 0..CONNECTION_PACKAGES {
            let lifetime = ExpirationData::new(CONNECTION_PACKAGE_EXPIRATION_DAYS);
            let connection_package_tbs = ConnectionPackageTbs::new(
                MlsInfraVersion::default(),
                key_store.connection_decryption_key.encryption_key(),
                lifetime,
                key_store.signing_key.credential().clone(),
            );
            let connection_package = connection_package_tbs.sign(&key_store.signing_key)?;
            connection_packages.push(connection_package);
        }

        let opaque_registration_record = OpaqueRegistrationRecord {
            client_message: client_registration_finish_result.message,
        };

        api_client
            .as_finish_user_registration(
                key_store.as_queue_decryption_key.encryption_key(),
                as_initial_ratchet_secret,
                connection_packages,
                opaque_registration_record,
                &key_store.signing_key,
            )
            .await?;

        // AS registration is complete, now create the user on the QS.
        let encrypted_client_credential = key_store
            .signing_key
            .credential()
            .encrypt(&key_store.client_credential_ear_key)?;

        let create_user_record_response = api_client
            .qs_create_user(
                key_store.friendship_token.clone(),
                key_store.qs_client_signing_key.verifying_key().clone(),
                key_store.qs_queue_decryption_key.encryption_key(),
                None,
                qs_initial_ratchet_secret,
                &key_store.qs_user_signing_key,
            )
            .await?;

        let mut user = Self {
            crypto_backend,
            api_clients,
            user_name,
            conversation_store: ConversationStore::default(),
            group_store: GroupStore::default(),
            key_store,
            qs_user_id: create_user_record_response.user_id,
            qs_client_id: create_user_record_response.client_id,
            qs_client_sequence_number_start: 0,
            as_client_sequence_number_start: 0,
            contacts: HashMap::default(),
            partial_contacts: HashMap::default(),
            notification_hub,
        };

        let mut qs_add_packages = vec![];
        for _ in 0..ADD_PACKAGES {
            // TODO: Which key do we need to use for encryption here? Probably
            // the client credential ear key, since friends need to be able to
            // decrypt it. We might want to use a separate key, though.
            let add_package = user.generate_add_package(&encrypted_client_credential, false)?;
            qs_add_packages.push(add_package);
        }
        let last_resort_add_package =
            user.generate_add_package(&encrypted_client_credential, true)?;
        qs_add_packages.push(last_resort_add_package);

        // Upload add packages
        let api_client = user.api_clients.get(&user.user_name().domain())?;
        api_client
            .qs_publish_key_packages(
                user.qs_client_id.clone(),
                qs_add_packages,
                user.key_store.add_package_ear_key.clone(),
                &user.key_store.qs_client_signing_key,
            )
            .await?;

        Ok(user)
    }

    pub(crate) fn generate_add_package(
        &mut self,
        encrypted_client_credential: &EncryptedClientCredential,
        last_resort: bool,
    ) -> Result<AddPackage> {
        let signature_ear_key = SignatureEarKey::random()?;
        let leaf_signer =
            InfraCredentialSigningKey::generate(&self.key_store.signing_key, &signature_ear_key);
        let credential_with_key = CredentialWithKey {
            credential: leaf_signer.credential().clone().into(),
            signature_key: leaf_signer.credential().verifying_key().clone(),
        };
        let capabilities = Capabilities::new(
            Some(&SUPPORTED_PROTOCOL_VERSIONS),
            Some(&SUPPORTED_CIPHERSUITES),
            Some(&SUPPORTED_EXTENSIONS),
            Some(&SUPPORTED_PROPOSALS),
            Some(&SUPPORTED_CREDENTIALS),
        );
        let client_reference = self.create_own_client_reference();
        let client_ref_extension = Extension::Unknown(
            QS_CLIENT_REFERENCE_EXTENSION_TYPE,
            UnknownExtension(client_reference.tls_serialize_detached()?),
        );
        let leaf_node_extensions = Extensions::single(client_ref_extension);
        let key_package_extensions = if last_resort {
            let last_resort_extension = Extension::LastResort(LastResortExtension::new());
            Extensions::single(last_resort_extension)
        } else {
            Extensions::default()
        };
        let kp = KeyPackage::builder()
            .key_package_extensions(key_package_extensions)
            .leaf_node_capabilities(capabilities)
            .leaf_node_extensions(leaf_node_extensions)
            .build(
                CryptoConfig {
                    ciphersuite: CIPHERSUITE,
                    version: ProtocolVersion::Mls10,
                },
                &self.crypto_backend,
                &leaf_signer,
                credential_with_key,
            )?;
        let esek = signature_ear_key.encrypt(&self.key_store.signature_ear_key_wrapper_key)?;
        self.key_store.leaf_signers.insert(
            leaf_signer.credential().verifying_key().clone(),
            (leaf_signer, signature_ear_key),
        );

        let add_package = AddPackage::new(kp.clone(), esek, encrypted_client_credential.clone());
        Ok(add_package)
    }

    /// Create new group
    pub async fn create_conversation(&mut self, title: &str) -> Result<Uuid> {
        let group_id = self
            .api_clients
            .default_client()?
            .ds_request_group_id()
            .await?;
        let client_reference = self.create_own_client_reference();
        let group = Group::create_group(
            &self.crypto_backend,
            &self.key_store.signing_key,
            group_id.clone(),
        )?;
        let partial_params = group.create_group_params(&self.crypto_backend)?;
        let encrypted_client_credential = self
            .key_store
            .signing_key
            .credential()
            .encrypt(group.credential_ear_key())?;
        let params = CreateGroupParamsOut {
            group_id: partial_params.group_id,
            ratchet_tree: partial_params.ratchet_tree,
            encrypted_client_credential,
            encrypted_signature_ear_key: partial_params.encrypted_signature_ear_key,
            creator_client_reference: client_reference,
            creator_user_auth_key: partial_params.user_auth_key,
            group_info: partial_params.group_info,
        };
        self.api_clients
            .default_client()?
            .ds_create_group(
                params,
                group.group_state_ear_key(),
                group.user_auth_key().ok_or(anyhow!("No user auth key"))?,
            )
            .await?;
        self.group_store.store_group(group)?;
        let attributes = ConversationAttributes {
            title: title.to_string(),
        };
        let conversation_id = self
            .conversation_store
            .create_group_conversation(group_id, attributes);
        self.notification_hub
            .dispatch_conversation_notification(conversation_id);
        Ok(conversation_id)
    }

    /// Invite users to an existing group
    pub async fn invite_users(
        &mut self,
        conversation_id: Uuid,
        invited_users: &[UserName],
    ) -> Result<()> {
        let conversation = self
            .conversation_store
            .conversation(conversation_id)
            .ok_or(anyhow!("Can't find conversation"))?;
        let group_id = conversation.group_id.clone();
        let owner_domain = conversation.owner_domain();
        let mut contact_add_infos: Vec<ContactAddInfos> = vec![];
        let mut contact_wai_keys = vec![];
        let mut client_credentials = vec![];
        for invited_user in invited_users {
            let user_name = invited_user.to_string().into();
            let contact = self
                .contacts
                .get_mut(&user_name)
                .ok_or(anyhow!("Couldn't find contact"))?;
            contact_wai_keys.push(contact.wai_ear_key().clone());
            client_credentials.push(contact.client_credentials());
            let add_info = if let Some(add_info) = contact.add_infos() {
                add_info
            } else {
                self.get_key_packages(&user_name).await?;
                let contact = self
                    .contacts
                    .get_mut(&user_name)
                    .ok_or(anyhow!("Can't find contact"))?;
                contact
                    .add_infos()
                    .ok_or(anyhow!("No add infos available for this contact"))?
            };
            contact_add_infos.push(add_info);
        }
        debug_assert!(contact_add_infos.len() == invited_users.len());
        let group = self
            .group_store
            .get_group_mut(&group_id.as_group_id())
            .ok_or(anyhow!("Can't find group"))?;
        // Adds new member and staged commit
        let params = group.invite(
            &self.crypto_backend,
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
        let conversation_messages = group.merge_pending_commit(&self.crypto_backend, None)?;
        // Send off the notifications
        self.send_off_notifications(conversation_id, conversation_messages)?;
        Ok(())
    }

    pub async fn remove_users(
        &mut self,
        conversation_id: Uuid,
        target_users: &[UserName],
    ) -> Result<()> {
        let conversation = self
            .conversation_store
            .conversation(conversation_id)
            .ok_or(anyhow!("Unknown conversation"))?;
        let group_id = &conversation.group_id;
        let group = self
            .group_store
            .get_group_mut(&group_id.as_group_id())
            .ok_or(anyhow!("Couldn't find group associated with conversation"))?;
        let mut clients = vec![];
        for user_name in target_users {
            let mut user_clients = group.user_client_ids(user_name);
            clients.append(&mut user_clients);
        }
        let params = group.remove(&self.crypto_backend, clients)?;
        let owner_domain = conversation.owner_domain();
        self.api_clients
            .get(&owner_domain)?
            .ds_remove_users(
                params,
                group.group_state_ear_key(),
                group.user_auth_key().ok_or(anyhow!("No user auth key"))?,
            )
            .await?;
        // Now that we know the commit went through, we can merge the commit and
        // create the events.
        let conversation_messages = group.merge_pending_commit(&self.crypto_backend, None)?;
        // Send off the notifications
        self.send_off_notifications(conversation_id, conversation_messages)?;
        Ok(())
    }

    fn send_off_notifications(
        &mut self,
        conversation_id: Uuid,
        conversation_messages: Vec<ConversationMessage>,
    ) -> Result<(), CorelibError> {
        for conversation_message in conversation_messages {
            let dispatched_conversation_message = DispatchedConversationMessage {
                conversation_id: UuidBytes::from_uuid(conversation_id),
                conversation_message: conversation_message.clone(),
            };
            self.conversation_store
                .store_message(conversation_id, conversation_message)?;
            self.notification_hub
                .dispatch_message_notification(dispatched_conversation_message);
        }
        Ok(())
    }

    /// Send a message and return it. Note that the message has already been
    /// sent to the DS and has internally been stored in the conversation store.
    pub async fn send_message(
        &mut self,
        conversation_id: Uuid,
        message: MessageContentType,
    ) -> Result<ConversationMessage> {
        let conversation = self
            .conversation_store
            .conversation(conversation_id)
            .ok_or(anyhow!("Unknown conversation"))?;
        let group_id = conversation.group_id.clone();
        // Generate ciphertext
        let params = self.group_store.create_message(
            &self.crypto_backend,
            &group_id.as_group_id(),
            message.clone(),
        )?;

        // Store message locally
        let message = Message::Content(ContentMessage {
            content: message,
            sender: self.user_name.to_string(),
        });

        // Send message to DS
        let owner_domain = conversation.owner_domain();
        self.api_clients
            .get(&owner_domain)?
            .ds_send_message(
                params,
                self.group_store
                    .leaf_signing_key(&group_id.as_group_id())
                    .ok_or(anyhow!("Can't find signing key for the given group"))?,
                self.group_store
                    .group_state_ear_key(&group_id.as_group_id())
                    .ok_or(anyhow!(
                        "Can't find group state ear key for the given group"
                    ))?,
            )
            .await?;

        let conversation_message = new_conversation_message(message);
        self.conversation_store
            .store_message(conversation_id, conversation_message.clone())
            .map_err(CorelibError::ConversationStore)?;
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
        let connection_packages = user_key_packages.connection_packages;
        // Verify the connection key packages
        log::info!("Verifying connection packages");
        let mut verified_connection_packages = vec![];
        for connection_package in connection_packages.into_iter() {
            let as_intermediate_credential = self
                .key_store
                .as_credentials
                .get(
                    &mut self.api_clients,
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
        let connection_group = Group::create_group(
            &self.crypto_backend,
            &self.key_store.signing_key,
            group_id.clone(),
        )?;
        let partial_params = connection_group.create_group_params(&self.crypto_backend)?;

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
        let params = CreateGroupParamsOut {
            group_id: partial_params.group_id,
            ratchet_tree: partial_params.ratchet_tree,
            encrypted_client_credential,
            encrypted_signature_ear_key: partial_params.encrypted_signature_ear_key,
            creator_client_reference: client_reference,
            creator_user_auth_key: partial_params.user_auth_key,
            group_info: partial_params.group_info,
        };
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
        self.group_store.store_group(connection_group)?;

        // Create the connection conversation
        let conversation_id = self.conversation_store.create_connection_conversation(
            group_id,
            user_name.clone(),
            ConversationAttributes {
                title: user_name.to_string(),
            },
        );

        let contact = PartialContact {
            user_name: user_name.clone(),
            conversation_id,
            friendship_package_ear_key,
        };
        self.partial_contacts.insert(user_name.clone(), contact);

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

        self.notification_hub
            .dispatch_conversation_notification(conversation_id);

        Ok(())
    }

    pub async fn update_user_key(&mut self, conversation_id: Uuid) -> Result<()> {
        let conversation = self
            .conversation_store
            .conversation(conversation_id)
            .ok_or(anyhow!("Unknown conversation"))?;
        let group = self
            .group_store
            .get_group_mut(&conversation.group_id.as_group_id())
            .ok_or(anyhow!("Couldn't find group associated with conversation"))?;
        let params = group.update_user_key(&self.crypto_backend)?;
        let owner_domain = conversation.owner_domain();
        self.api_clients
            .get(&owner_domain)?
            .ds_update_client(params, group.group_state_ear_key(), group.leaf_signer())
            .await?;
        let conversation_messages = group.merge_pending_commit(&self.crypto_backend, None)?;
        self.send_off_notifications(conversation_id, conversation_messages)?;
        Ok(())
    }

    pub async fn delete_group(&mut self, conversation_id: Uuid) -> Result<()> {
        let conversation = self
            .conversation_store
            .conversation(conversation_id)
            .ok_or(anyhow!("Unknown conversation"))?;
        let group = self
            .group_store
            .get_group_mut(&conversation.group_id.as_group_id())
            .ok_or(anyhow!("Couldn't find group associated with conversation"))?;
        let past_members: Vec<_> = group.members().into_iter().map(|m| m.to_string()).collect();
        // No need to send a message to the server if we are the only member.
        // TODO: Make sure this is what we want.
        if past_members.len() != 1 {
            let params = group.delete(&self.crypto_backend)?;
            let owner_domain = conversation.owner_domain();
            self.api_clients
                .get(&owner_domain)?
                .ds_delete_group(
                    params,
                    group.user_auth_key().ok_or(anyhow!("No user auth key"))?,
                    group.group_state_ear_key(),
                )
                .await?;
            let conversation_messages = group.merge_pending_commit(&self.crypto_backend, None)?;
            self.send_off_notifications(conversation_id, conversation_messages)?;
        }
        self.conversation_store
            .set_inactive(conversation_id, &past_members);
        Ok(())
    }

    pub async fn as_fetch_messages(&mut self) -> Result<Vec<QueueMessage>> {
        let mut remaining_messages = 1;
        let mut messages: Vec<QueueMessage> = Vec::new();
        while remaining_messages > 0 {
            let mut response = self
                .api_clients
                .default_client()?
                .as_dequeue_messages(
                    self.as_client_sequence_number_start,
                    1_000_000,
                    &self.key_store.signing_key,
                )
                .await?;

            if let Some(message) = messages.last() {
                self.as_client_sequence_number_start = message.sequence_number;
            }

            remaining_messages = response.remaining_messages_number;
            messages.append(&mut response.messages);
        }
        Ok(messages)
    }

    pub async fn qs_fetch_messages(&mut self) -> Result<Vec<QueueMessage>> {
        let mut remaining_messages = 1;
        let mut messages: Vec<QueueMessage> = Vec::new();
        while remaining_messages > 0 {
            let mut response = self
                .api_clients
                .default_client()?
                .qs_dequeue_messages(
                    &self.qs_client_id,
                    self.qs_client_sequence_number_start,
                    1_000_000,
                    &self.key_store.qs_client_signing_key,
                )
                .await?;

            if let Some(message) = messages.last() {
                self.as_client_sequence_number_start = message.sequence_number;
            }

            remaining_messages = response.remaining_messages_number;
            messages.append(&mut response.messages);
        }
        Ok(messages)
    }

    pub async fn leave_group(&mut self, conversation_id: Uuid) -> Result<()> {
        let conversation = self
            .conversation_store
            .conversation(conversation_id)
            .ok_or(anyhow!("Unknown conversation"))?;
        let group = self
            .group_store
            .get_group_mut(&conversation.group_id.as_group_id())
            .ok_or(anyhow!("Couldn't find group associated with conversation"))?;
        let params = group.leave_group(&self.crypto_backend)?;
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

    pub async fn update(&mut self, conversation_id: Uuid) -> Result<()> {
        let conversation = self
            .conversation_store
            .conversation(conversation_id)
            .ok_or(anyhow!("Unknown conversation"))?;
        let group = self
            .group_store
            .get_group_mut(&conversation.group_id.as_group_id())
            .ok_or(anyhow!("Couldn't find group associated with conversation"))?;
        let params = group.update(&self.crypto_backend)?;
        let owner_domain = conversation.owner_domain();
        self.api_clients
            .get(&owner_domain)?
            .ds_update_client(params, group.group_state_ear_key(), group.leaf_signer())
            .await?;
        let conversation_messages = group.merge_pending_commit(&self.crypto_backend, None)?;
        self.send_off_notifications(conversation_id, conversation_messages)?;
        Ok(())
    }

    pub fn contacts(&self) -> Vec<Contact> {
        self.contacts.values().cloned().collect()
    }

    pub fn partial_contacts(&self) -> Vec<PartialContact> {
        self.partial_contacts.values().cloned().collect()
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

    pub fn user_name(&self) -> &UserName {
        &self.user_name
    }

    fn group(&self, conversation_id: Uuid) -> Option<&Group> {
        self.conversation(conversation_id)
            .and_then(|conversation| self.group_store.group(&conversation.group_id.as_group_id()))
    }

    /// Returns None if there is no conversation with the given id.
    pub fn group_members(&self, conversation_id: Uuid) -> Option<Vec<UserName>> {
        self.group(conversation_id).map(|group| {
            group
                .members()
                .iter()
                .map(|member| member.clone())
                .collect()
        })
    }

    pub fn pending_removes(&self, conversation_id: Uuid) -> Option<Vec<UserName>> {
        self.group(conversation_id).map(|group| {
            group
                .mls_group()
                .pending_proposals()
                .filter_map(|proposal| match proposal.proposal() {
                    Proposal::Remove(rp) => group
                        .client_by_index(rp.removed().usize())
                        .map(|c| c.user_name()),
                    _ => None,
                })
                .collect()
        })
    }

    pub async fn websocket(&mut self, timeout: u64, retry_interval: u64) -> Result<QsWebSocket> {
        let api_client = self.api_clients.default_client();
        Ok(api_client?
            .spawn_websocket(self.qs_client_id.clone(), timeout, retry_interval)
            .await?)
    }

    pub fn conversations(&self) -> Vec<Conversation> {
        self.conversation_store.conversations()
    }

    pub(crate) async fn get_key_packages(&mut self, contact_name: &UserName) -> Result<()> {
        let qs_verifying_key = self.qs_verifying_key(&contact_name.domain()).await?.clone();
        let contact = self
            .contacts
            .get_mut(contact_name)
            .ok_or(anyhow!("Can't find contact"))?;
        let mut add_infos = Vec::new();
        for _ in 0..5 {
            let response = self
                .api_clients
                .get(&contact_name.domain())?
                .qs_key_package_batch(
                    contact.friendship_token.clone(),
                    contact.add_package_ear_key.clone(),
                )
                .await?;
            let key_packages: Vec<(KeyPackage, SignatureEarKey)> = response
                .add_packages
                .into_iter()
                .map(|add_package| {
                    let validated_add_package = add_package
                        .validate(self.crypto_backend.crypto(), ProtocolVersion::default())?;
                    let key_package = validated_add_package.key_package().clone();
                    let sek = SignatureEarKey::decrypt(
                        &contact.signature_ear_key_wrapper_key,
                        validated_add_package.encrypted_signature_ear_key(),
                    )?;
                    Ok((key_package, sek))
                })
                .collect::<Result<Vec<_>>>()?;
            let add_info = ContactAddInfos {
                key_packages,
                key_package_batch: response.key_package_batch.verify(&qs_verifying_key)?,
            };
            add_infos.push(add_info);
        }
        contact.add_infos.append(&mut add_infos);
        Ok(())
    }
}
