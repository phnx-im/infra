// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::net::SocketAddr;

use opaque_ke::{
    ClientRegistration, ClientRegistrationFinishParameters, ClientRegistrationFinishResult,
    ClientRegistrationStartResult, Identifiers,
};
use phnxapiclient::{ApiClient, TransportEncryption};
use phnxbackend::{
    auth_service::{
        credentials::{
            keys::{ClientSigningKey, InfraCredentialSigningKey},
            AsCredential, AsIntermediateCredential, ClientCredential, ClientCredentialCsr,
            ClientCredentialPayload, ExpirationData,
        },
        AsClientId, OpaqueRegistrationRecord, OpaqueRegistrationRequest, UserName,
    },
    crypto::{
        ear::{
            keys::{
                AddPackageEarKey, ClientCredentialEarKey, PushTokenEarKey, SignatureEarKey,
                WelcomeAttributionInfoEarKey,
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
    messages::{
        client_as::{
            AsQueueRatchet, ConnectionEstablishmentPackageTbs, ConnectionPackage,
            ConnectionPackageTbs, FriendshipPackage, UserKeyPackagesParams,
        },
        client_ds::QsQueueRatchet,
        FriendshipToken, MlsInfraVersion,
    },
    qs::{AddPackage, ClientIdEncryptionKey, PushToken, QsClientId, QsUserId, QsVerifyingKey},
};
use rand::rngs::OsRng;

use crate::contacts::{Contact, ContactAddInfos, PartialContact};

use super::*;

pub mod process;

pub(crate) const CIPHERSUITE: Ciphersuite =
    Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;

pub(crate) const CONNECTION_PACKAGES: usize = 50;
pub(crate) const ADD_PACKAGES: usize = 50;
pub(crate) const CONNECTION_PACKAGE_EXPIRATION_DAYS: i64 = 30;

pub(crate) struct MemoryUserKeyStore {
    // Client credential secret key
    signing_key: ClientSigningKey,
    // AS-specific key material
    as_queue_decryption_key: RatchetDecryptionKey,
    as_queue_ratchet: AsQueueRatchet,
    connection_decryption_key: ConnectionDecryptionKey,
    as_credentials: Vec<AsCredential>,
    as_intermediate_credentials: Vec<AsIntermediateCredential>,
    // QS-specific key material
    qs_client_signing_key: QsClientSigningKey,
    qs_user_signing_key: QsUserSigningKey,
    qs_queue_decryption_key: RatchetDecryptionKey,
    qs_queue_ratchet: QsQueueRatchet,
    qs_verifying_key: QsVerifyingKey,
    qs_client_id_encryption_key: ClientIdEncryptionKey,
    push_token_ear_key: PushTokenEarKey,
    // These are keys that we send to our contacts
    friendship_token: FriendshipToken,
    add_package_ear_key: AddPackageEarKey,
    client_credential_ear_key: ClientCredentialEarKey,
    signature_ear_key: SignatureEarKey,
    wai_ear_key: WelcomeAttributionInfoEarKey,
    // Leaf credentials in KeyPackages the active ones are stored in the group
    // that they belong to.
    leaf_signers: HashMap<SignaturePublicKey, InfraCredentialSigningKey>,
}

pub struct SelfUser {
    pub(crate) crypto_backend: OpenMlsRustCrypto,
    pub(crate) api_client: ApiClient,
    pub(crate) user_name: UserName,
    pub(crate) qs_user_id: QsUserId,
    pub(crate) qs_client_id: QsClientId,
    pub(crate) conversation_store: ConversationStore,
    pub(crate) group_store: GroupStore,
    pub(crate) key_store: MemoryUserKeyStore,
    pub(crate) contacts: HashMap<UserName, Contact>,
    pub(crate) partial_contacts: HashMap<UserName, PartialContact>,
}

impl SelfUser {
    /// Create a new user with the given name and a fresh set of credentials.
    pub async fn new(user_name: &str, password: &str, address: SocketAddr) -> Self {
        log::info!("Creating new user {}", user_name);
        let user_name: UserName = UserName::from(user_name.to_string());
        let crypto_backend = OpenMlsRustCrypto::default();
        // Let's turn TLS off for now.
        let api_client = ApiClient::initialize(address, TransportEncryption::Off).unwrap();

        let as_client_id = AsClientId::random(crypto_backend.rand(), user_name.clone()).unwrap();
        let (client_credential_csr, prelim_signing_key) =
            ClientCredentialCsr::new(as_client_id, SignatureScheme::ED25519).unwrap();

        log::info!("Fetch AS credentials");

        let as_credentials_response = api_client.as_as_credentials().await.unwrap();
        log::info!("Verifying credentials");
        let as_intermediate_credentials: Vec<AsIntermediateCredential> = as_credentials_response
            .as_intermediate_credentials
            .into_iter()
            .map(|as_inter_cred| {
                let as_credential = as_credentials_response
                    .as_credentials
                    .iter()
                    .find(|as_cred| &as_cred.fingerprint().unwrap() == as_inter_cred.fingerprint())
                    .unwrap();
                as_inter_cred.verify(as_credential.verifying_key()).unwrap()
            })
            .collect();

        let chosen_inter_credential = as_intermediate_credentials.first().unwrap();

        let client_credential_payload = ClientCredentialPayload::new(
            client_credential_csr,
            None,
            chosen_inter_credential.fingerprint().unwrap(),
        );

        // Let's do OPAQUE registration.
        // First get the server setup information.
        let mut client_rng = OsRng;
        let client_registration_start_result: ClientRegistrationStartResult<OpaqueCiphersuite> =
            ClientRegistration::<OpaqueCiphersuite>::start(&mut client_rng, password.as_bytes())
                .unwrap();

        let opaque_registration_request = OpaqueRegistrationRequest {
            client_message: client_registration_start_result.message,
        };

        // Register the user with the backend.
        let response = api_client
            .as_initiate_create_user(client_credential_payload, opaque_registration_request)
            .await
            .unwrap();

        // Complete the OPAQUE registration.
        let address = api_client.address().clone().to_string();
        let identifiers = Identifiers {
            client: Some(user_name.as_bytes()),
            server: Some(address.as_bytes()),
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
                .unwrap();

        let credential: ClientCredential = response
            .client_credential
            .verify(chosen_inter_credential.verifying_key())
            .unwrap();

        let signing_key =
            ClientSigningKey::from_prelim_key(prelim_signing_key, credential).unwrap();
        let as_queue_decryption_key = RatchetDecryptionKey::generate().unwrap();
        let as_initial_ratchet_secret = RatchetSecret::random().unwrap();
        let qs_queue_decryption_key = RatchetDecryptionKey::generate().unwrap();
        let qs_initial_ratchet_secret = RatchetSecret::random().unwrap();
        let qs_client_signing_key = QsClientSigningKey::random().unwrap();
        let qs_user_signing_key = QsUserSigningKey::random().unwrap();

        let leaf_signers = HashMap::new();
        // TODO: The following five keys should be derived from a single
        // friendship key. Once that's done, remove the random constructors.
        let friendship_token = FriendshipToken::random().unwrap();
        let add_package_ear_key = AddPackageEarKey::random().unwrap();
        let client_credential_ear_key = ClientCredentialEarKey::random().unwrap();
        let signature_ear_key = SignatureEarKey::random().unwrap();
        let wai_ear_key: WelcomeAttributionInfoEarKey =
            WelcomeAttributionInfoEarKey::random().unwrap();
        let push_token_ear_key = PushTokenEarKey::random().unwrap();
        let qs_verifying_key = api_client.qs_verifying_key().await.unwrap().verifying_key;
        let qs_encryption_key = api_client.qs_encryption_key().await.unwrap().encryption_key;
        let connection_decryption_key = ConnectionDecryptionKey::generate().unwrap();

        // Mutable, because we need to access the leaf signers later.
        let mut key_store = MemoryUserKeyStore {
            signing_key,
            as_queue_decryption_key,
            as_queue_ratchet: as_initial_ratchet_secret.clone().try_into().unwrap(),
            connection_decryption_key,
            as_credentials: as_credentials_response.as_credentials,
            as_intermediate_credentials,
            qs_client_signing_key,
            qs_user_signing_key,
            qs_queue_decryption_key,
            qs_queue_ratchet: qs_initial_ratchet_secret.clone().try_into().unwrap(),
            qs_verifying_key,
            push_token_ear_key,
            friendship_token,
            add_package_ear_key,
            client_credential_ear_key,
            signature_ear_key,
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
            let connection_package = connection_package_tbs.sign(&key_store.signing_key).unwrap();
            connection_packages.push(connection_package);
        }

        let opaque_registration_record = OpaqueRegistrationRecord {
            client_message: client_registration_finish_result.message,
        };

        api_client
            .as_finish_user_registration(
                user_name.clone(),
                key_store.as_queue_decryption_key.encryption_key(),
                as_initial_ratchet_secret,
                connection_packages,
                opaque_registration_record,
                &key_store.signing_key,
            )
            .await
            .unwrap();

        // AS registration is complete, now create the user on the QS.
        let encrypted_client_credential = key_store
            .signing_key
            .credential()
            .encrypt(&key_store.client_credential_ear_key)
            .unwrap();
        let mut qs_add_packages = vec![];
        for _ in 0..ADD_PACKAGES {
            // TODO: Which key do we need to use for encryption here? Probably
            // the client credential ear key, since friends need to be able to
            // decrypt it. We might want to use a separate key, though.
            let (kp, leaf_signer) = SelfUser::generate_keypackage(
                &crypto_backend,
                &key_store.signing_key,
                &key_store.signature_ear_key,
            );
            key_store.leaf_signers.insert(
                leaf_signer.credential().verifying_key().clone(),
                leaf_signer,
            );
            let add_package = AddPackage::new(kp.clone(), encrypted_client_credential.clone());
            qs_add_packages.push(add_package);
        }

        let push_token = PushToken::dummy();
        let encrypted_push_token = push_token.encrypt(&key_store.push_token_ear_key).unwrap();

        let create_user_record_response = api_client
            .qs_create_user(
                key_store.friendship_token.clone(),
                key_store.qs_client_signing_key.verifying_key().clone(),
                key_store.qs_queue_decryption_key.encryption_key(),
                qs_add_packages,
                key_store.add_package_ear_key.clone(),
                Some(encrypted_push_token),
                qs_initial_ratchet_secret,
                &key_store.qs_user_signing_key,
            )
            .await
            .unwrap();

        Self {
            crypto_backend,
            api_client,
            user_name,
            conversation_store: ConversationStore::default(),
            group_store: GroupStore::default(),
            key_store,
            qs_user_id: create_user_record_response.user_id,
            qs_client_id: create_user_record_response.client_id,
            contacts: HashMap::default(),
            partial_contacts: HashMap::default(),
        }
    }

    pub(crate) fn generate_keypackage(
        crypto_backend: &impl OpenMlsCryptoProvider,
        signing_key: &ClientSigningKey,
        signature_encryption_key: &SignatureEarKey,
    ) -> (KeyPackage, InfraCredentialSigningKey) {
        let leaf_signer =
            InfraCredentialSigningKey::generate(signing_key, signature_encryption_key);
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
        let kp = KeyPackage::builder()
            .leaf_node_capabilities(capabilities)
            .build(
                CryptoConfig {
                    ciphersuite: CIPHERSUITE,
                    version: ProtocolVersion::Mls10,
                },
                crypto_backend,
                &leaf_signer,
                credential_with_key,
            )
            .unwrap();
        (kp, leaf_signer)
    }

    /// Create new group
    pub async fn create_conversation(&mut self, title: &str) -> Result<Uuid, CorelibError> {
        let group_id = self.api_client.ds_request_group_id().await.unwrap();
        self.group_store.create_group(
            &self.crypto_backend,
            &self.key_store.signing_key,
            group_id.clone(),
        );
        let conversation_id = Uuid::new_v4();
        let attributes = ConversationAttributes {
            title: title.to_string(),
        };
        self.conversation_store
            .create_group_conversation(conversation_id, group_id, attributes);
        Ok(conversation_id)
    }

    /// Invite users to an existing group
    pub async fn invite_users<T: Notifiable>(
        &mut self,
        conversation_id: &Uuid,
        invited_users: Vec<&str>,
        // For now, we're  passing this in. It might be better to pass the
        // necessary data back out instead.
        notification_hub: &mut NotificationHub<T>,
    ) -> Result<(), CorelibError> {
        let conversation = self
            .conversation_store
            .conversation(conversation_id)
            .unwrap();
        let group_id = &conversation.group_id;
        let group = self
            .group_store
            .get_group_mut(&group_id.as_group_id())
            .unwrap();
        let mut contact_add_infos: Vec<ContactAddInfos> = vec![];
        let mut contact_wai_keys = vec![];
        let mut client_credentials = vec![];
        for invited_user in invited_users {
            let user_name = invited_user.to_string().into();
            let contact = self.contacts.get_mut(&user_name).unwrap();
            contact_add_infos.push(contact.add_infos());
            contact_wai_keys.push(contact.wai_ear_key().clone());
            client_credentials.push(contact.client_credentials());
        }
        // Adds new member and staged commit
        let params = group
            .invite(
                &self.crypto_backend,
                &self.key_store.signing_key,
                contact_add_infos,
                contact_wai_keys,
                client_credentials,
            )
            .map_err(CorelibError::Group)?;
        let staged_commit = group.pending_commit().unwrap();
        // We're not getting a response, but if it's not an error, the commit
        // must have gone through.
        self.api_client
            .ds_add_users(params, group.group_state_ear_key(), group.user_auth_key())
            .await
            .unwrap();
        // Now that we know the commit went through, we can merge the commit and
        // create the events.
        let conversation_messages =
            staged_commit_to_conversation_messages(&self.user_name, staged_commit);
        // Merge the pending commit.
        group.merge_pending_commit(&self.crypto_backend)?;
        // Send off the notifications
        for conversation_message in conversation_messages {
            let dispatched_conversation_message = DispatchedConversationMessage {
                conversation_id: conversation_id.to_owned(),
                conversation_message: conversation_message.clone(),
            };
            self.conversation_store
                .store_message(conversation_id, conversation_message)?;
            notification_hub.dispatch_message_notification(dispatched_conversation_message);
        }
        Ok(())
    }

    pub async fn remove_users<T: Notifiable>(
        &mut self,
        conversation_id: &Uuid,
        target_users: Vec<&str>,
        // For now, we're  passing this in. It might be better to pass the
        // necessary data back out instead.
        notification_hub: &mut NotificationHub<T>,
    ) -> Result<(), CorelibError> {
        let conversation = self
            .conversation_store
            .conversation(conversation_id)
            .unwrap();
        let group_id = &conversation.group_id;
        let group = self
            .group_store
            .get_group_mut(&group_id.as_group_id())
            .unwrap();
        let mut clients = vec![];
        for user_name in target_users {
            let user_name = user_name.to_string().into();
            let contact = self.contacts.get(&user_name).unwrap();
            let mut contact_clients = contact
                .client_credentials()
                .iter()
                .map(|credential| credential.identity())
                .collect();
            clients.append(&mut contact_clients);
        }
        let params = group.remove(&self.crypto_backend, clients).unwrap();
        self.api_client
            .ds_remove_users(params, group.group_state_ear_key(), group.user_auth_key())
            .await
            .unwrap();
        let staged_commit = group.pending_commit().unwrap();
        // Now that we know the commit went through, we can merge the commit and
        // create the events.
        let conversation_messages =
            staged_commit_to_conversation_messages(&self.user_name, staged_commit);
        // Merge the pending commit.
        group.merge_pending_commit(&self.crypto_backend)?;
        // Send off the notifications
        for conversation_message in conversation_messages {
            let dispatched_conversation_message = DispatchedConversationMessage {
                conversation_id: conversation_id.to_owned(),
                conversation_message: conversation_message.clone(),
            };
            self.conversation_store
                .store_message(conversation_id, conversation_message)?;
            notification_hub.dispatch_message_notification(dispatched_conversation_message);
        }
        Ok(())
    }

    /// Send a message and return it. Note that the message has already been
    /// sent to the DS and has internally been stored in the conversation store.
    pub async fn send_message(
        &mut self,
        conversation_id: Uuid,
        message: &str,
    ) -> Result<ConversationMessage, CorelibError> {
        let group_id = &self
            .conversation_store
            .conversation(&conversation_id)
            .unwrap()
            .group_id
            .clone();
        // Generate ciphertext
        let params = self
            .group_store
            .create_message(&self.crypto_backend, &group_id.as_group_id(), message)
            .map_err(CorelibError::Group)?;

        // Store message locally
        let message = Message::Content(ContentMessage {
            content: MessageContentType::Text(TextMessage {
                message: message.to_string(),
            }),
            sender: self.user_name.as_bytes().to_vec(),
        });
        let conversation_message = new_conversation_message(message);
        self.conversation_store
            .store_message(&conversation_id, conversation_message.clone())
            .map_err(CorelibError::ConversationStore)?;

        // Send message to DS
        self.api_client
            .ds_send_message(
                params,
                &self.group_store.leaf_signing_key(&group_id.as_group_id()),
                &self
                    .group_store
                    .group_state_ear_key(&group_id.as_group_id()),
            )
            .await
            .unwrap();
        Ok(conversation_message)
    }

    pub async fn add_contact(&mut self, user_name: &str) {
        let user_name: UserName = user_name.to_string().into();
        let params = UserKeyPackagesParams {
            user_name: user_name.clone(),
        };
        // First we fetch connection key packages from the AS, then we establish
        // a connection group. Finally, we fully add the user as a contact.
        let user_key_packages = self.api_client.as_user_key_packages(params).await.unwrap();
        let connection_packages = user_key_packages.key_packages;
        // Verify the connection key packages
        let verified_connection_packages: Vec<ConnectionPackage> = connection_packages
            .into_iter()
            .map(|cp| {
                let verifying_key = self
                    .key_store
                    .as_intermediate_credentials
                    .iter()
                    .find_map(|as_intermediate_credential| {
                        if &as_intermediate_credential.fingerprint().unwrap()
                            == cp.client_credential_signer_fingerprint()
                        {
                            Some(as_intermediate_credential.verifying_key())
                        } else {
                            None
                        }
                    })
                    .unwrap();
                cp.verify(verifying_key).unwrap()
            })
            .collect();

        // TODO: Connection Package Validation
        // * Version
        // * Lifetime

        // Get a group id for the connection group
        let group_id = self.api_client.ds_request_group_id().await.unwrap();
        // Create the connection group
        let connection_group = Group::create_group(
            &self.crypto_backend,
            &self.key_store.signing_key,
            group_id.clone(),
        );

        // TODO: Once we allow multi-client, invite all our other clients to the
        // connection group.

        let friendship_package = FriendshipPackage {
            friendship_token: self.key_store.friendship_token.clone(),
            add_package_ear_key: self.key_store.add_package_ear_key.clone(),
            client_credential_ear_key: self.key_store.client_credential_ear_key.clone(),
            signature_ear_key: self.key_store.signature_ear_key.clone(),
            wai_ear_key: self.key_store.wai_ear_key.clone(),
        };

        // Create a connection establishment package
        let connection_establishment_package = ConnectionEstablishmentPackageTbs {
            sender_client_credential: self.key_store.signing_key.credential().clone(),
            connection_group_id: group_id.clone(),
            connection_group_ear_key: connection_group.group_state_ear_key().clone(),
            connection_group_credential_key: connection_group.credential_ear_key().clone(),
            connection_group_signature_ear_key: connection_group.signature_ear_key().clone(),
            friendship_package,
        }
        .sign(&self.key_store.signing_key)
        .unwrap();

        self.group_store.store_group(connection_group).unwrap();
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
        };
        self.partial_contacts.insert(user_name, contact);

        // Encrypt the connection establishment package for each connection and send it off.
        for connection_package in verified_connection_packages {
            let ciphertext = connection_establishment_package.encrypt(
                connection_package.encryption_key(),
                &[],
                &[],
            );
            let client_id = connection_package.client_credential().identity();

            self.api_client
                .as_enqueue_message(client_id, ciphertext)
                .await
                .unwrap();
        }
    }
}
