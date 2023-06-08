// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use futures::executor::block_on;
use opaque_ke::{
    ClientRegistration, ClientRegistrationFinishParameters, ClientRegistrationFinishResult,
    ClientRegistrationStartResult, Identifiers,
};
use phnxapiclient::{ApiClient, TransportEncryption};
use phnxbackend::{
    auth_service::{
        credentials::{
            keys::{ClientSigningKey, InfraCredentialSigningKey},
            AsIntermediateCredential, ClientCredential, ClientCredentialCsr,
            ClientCredentialPayload, ExpirationData,
        },
        AsClientId, OpaqueRegistrationRecord, OpaqueRegistrationRequest, UserName,
    },
    crypto::{
        ear::{
            keys::{AddPackageEarKey, ClientCredentialEarKey, PushTokenEarKey, SignatureEarKey},
            EarEncryptable,
        },
        signatures::{
            keys::{QsClientSigningKey, QsUserSigningKey},
            signable::{Signable, Verifiable},
        },
        ConnectionDecryptionKey, OpaqueCiphersuite, QueueRatchet, RatchetDecryptionKey,
    },
    messages::{client_as::ConnectionPackageTbs, FriendshipToken, MlsInfraVersion},
    qs::{AddPackage, PushToken, QsClientId, QsUserId},
};
use rand::rngs::OsRng;

use crate::contacts::Contact;

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
    as_ratchet_key: QueueRatchet,
    // QS-specific key material
    qs_client_signing_key: QsClientSigningKey,
    qs_user_signing_key: QsUserSigningKey,
    qs_queue_decryption_key: RatchetDecryptionKey,
    qs_ratchet_key: QueueRatchet,
    friendship_token: FriendshipToken,
    add_package_ear_key: AddPackageEarKey,
    client_credential_ear_key: ClientCredentialEarKey,
    signature_ear_key: SignatureEarKey,
    push_token_ear_key: PushTokenEarKey,
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
}

impl SelfUser {
    /// Create a new user with the given name and a fresh set of credentials.
    pub fn new(user_name: String, password: String) -> Self {
        let user_name: UserName = user_name.into();
        let crypto_backend = OpenMlsRustCrypto::default();
        // Let's turn TLS off for now.
        let api_client = ApiClient::initialize(
            "http://localhost:8000".to_string(),
            TransportEncryption::Off,
        )
        .unwrap();

        let as_client_id = AsClientId::random(crypto_backend.rand(), user_name).unwrap();
        let (client_credential_csr, prelim_signing_key) =
            ClientCredentialCsr::new(as_client_id, SignatureScheme::ED25519).unwrap();

        let as_credentials = block_on(api_client.as_as_credentials()).unwrap();
        let verifiable_intemediate_credential =
            as_credentials.as_intermediate_credentials.pop().unwrap();
        let as_credential = as_credentials
            .as_credentials
            .into_iter()
            .find(|as_cred| {
                &as_cred.fingerprint().unwrap() == verifiable_intemediate_credential.fingerprint()
            })
            .unwrap();
        let as_intermediate_credential: AsIntermediateCredential =
            verifiable_intemediate_credential
                .verify(as_credential.verifying_key())
                .unwrap();

        let client_credential_payload = ClientCredentialPayload::new(
            client_credential_csr,
            None,
            as_intermediate_credential.fingerprint().unwrap(),
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
        let response = block_on(
            api_client
                .as_initiate_create_user(client_credential_payload, opaque_registration_request),
        )
        .unwrap();

        // Complete the OPAQUE registration.
        let identifiers = Identifiers {
            client: Some(user_name.as_bytes()),
            server: Some(api_client.base_url().as_bytes()),
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
            .verify(as_intermediate_credential.verifying_key())
            .unwrap();

        let signing_key =
            ClientSigningKey::from_prelim_key(prelim_signing_key, credential).unwrap();
        let as_queue_decryption_key = RatchetDecryptionKey::generate().unwrap();
        let as_ratchet_key = QueueRatchet::random().unwrap();
        let qs_queue_decryption_key = RatchetDecryptionKey::generate().unwrap();
        let qs_ratchet_key = QueueRatchet::random().unwrap();
        let qs_client_signing_key = QsClientSigningKey::random().unwrap();
        let qs_user_signing_key = QsUserSigningKey::random().unwrap();

        let leaf_signers = HashMap::new();
        // TODO: The following four keys should be derived from a single
        // friendship key. Once that's done, remove the random constructors.
        let friendship_token = FriendshipToken::random().unwrap();
        let add_package_ear_key = AddPackageEarKey::random().unwrap();
        let client_credential_ear_key = ClientCredentialEarKey::random().unwrap();
        let signature_ear_key = SignatureEarKey::random().unwrap();
        let push_token_ear_key = PushTokenEarKey::random().unwrap();

        let key_store = MemoryUserKeyStore {
            signing_key,
            as_queue_decryption_key,
            as_ratchet_key,
            qs_client_signing_key,
            qs_user_signing_key,
            qs_queue_decryption_key,
            qs_ratchet_key,
            friendship_token,
            add_package_ear_key,
            client_credential_ear_key,
            signature_ear_key,
            push_token_ear_key,
            leaf_signers,
        };

        // TODO: We need another leaf credential type in OpenMLS for connection
        // key packages. Should we use the client credentials directly?
        let mut connection_packages = vec![];
        for _ in 0..CONNECTION_PACKAGES {
            let decryption_key = ConnectionDecryptionKey::generate().unwrap();
            let lifetime = ExpirationData::new(CONNECTION_PACKAGE_EXPIRATION_DAYS);
            let connection_package_tbs = ConnectionPackageTbs::new(
                MlsInfraVersion::default(),
                decryption_key.encryption_key(),
                lifetime,
                key_store.signing_key.credential().clone(),
            );
            let connection_package = connection_package_tbs.sign(&key_store.signing_key).unwrap();
            connection_packages.push(connection_package);
        }

        let opaque_registration_record = OpaqueRegistrationRecord {
            client_message: client_registration_finish_result.message,
        };

        block_on(api_client.as_finish_user_registration(
            user_name.into(),
            key_store.as_queue_decryption_key.encryption_key().clone(),
            as_ratchet_key,
            connection_packages,
            opaque_registration_record,
            &key_store.signing_key,
        ))
        .unwrap();

        // AS registration is complete, now create the user on the QS.
        let icc_ciphertext = signing_key
            .credential()
            .encrypt(&client_credential_ear_key)
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
            let add_package = AddPackage::new(kp.clone(), icc_ciphertext);
            qs_add_packages.push(add_package);
        }

        let push_token = PushToken::dummy();
        let encrypted_push_token = push_token.encrypt(&push_token_ear_key).unwrap();

        let create_user_record_response = block_on(api_client.qs_create_user(
            key_store.friendship_token.clone(),
            key_store.qs_client_signing_key.verifying_key().clone(),
            key_store.qs_queue_decryption_key.encryption_key().clone(),
            qs_add_packages,
            key_store.add_package_ear_key.clone(),
            Some(encrypted_push_token),
            key_store.qs_ratchet_key.clone(),
            &qs_user_signing_key,
        ))
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
        let kp = KeyPackage::builder()
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
    pub fn create_conversation(&mut self, title: &str) -> Result<Uuid, CorelibError> {
        let group_id = block_on(self.api_client.ds_request_group_id()).unwrap();
        self.group_store
            .create_group(&self.crypto_backend, &self.key_store.signing_key, group_id);
        let conversation_id = Uuid::new_v4();
        let attributes = ConversationAttributes {
            title: title.to_string(),
        };
        self.conversation_store
            .create_group_conversation(conversation_id, group_id, attributes);
        Ok(conversation_id)
    }

    /// Invite users to an existing group
    pub fn invite_users<T: Notifiable>(
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
        let group = self.group_store.get_group_mut(group_id).unwrap();
        let invited_contacts = invited_users
            .iter()
            .map(|u| {
                let user_name = u.to_string().into();
                self.contacts.get(&user_name).unwrap()
            })
            .collect();
        // Adds new member and staged commit
        let (params, staged_commit) = group
            .invite(
                &self.crypto_backend,
                &self.key_store.signing_key,
                invited_contacts,
            )
            .map_err(CorelibError::Group)?;
        // We're not getting a response, but if it's not an error, the commit
        // must have gone through.
        block_on(self.api_client.ds_add_users(
            params,
            group.group_state_ear_key(),
            group.user_auth_key(),
        ))
        .unwrap();
        // Now that we know the commit went through, we can merge the commit and
        // create the events.
        group.merge_pending_commit(&self.crypto_backend)?;
        let conversation_messages =
            staged_commit_to_conversation_messages(&self.user_name, staged_commit);
        for conversation_message in conversation_messages {
            let dispatched_conversation_message = DispatchedConversationMessage {
                conversation_id: conversation_id.clone(),
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
    pub fn send_message(
        &mut self,
        conversation_id: Uuid,
        message: &str,
    ) -> Result<ConversationMessage, CorelibError> {
        let group_id = &self
            .conversation_store
            .conversation(&conversation_id)
            .unwrap()
            .group_id;
        // Generate ciphertext
        let params = self
            .group_store
            .create_message(&self.crypto_backend, group_id, message)
            .map_err(CorelibError::Group)?;

        // Store message locally
        let message = Message::Content(ContentMessage {
            content: MessageContentType::Text(TextMessage {
                message: message.to_string(),
            }),
            sender: self.user_name.clone(),
        });
        let conversation_message = new_conversation_message(message);
        self.conversation_store
            .store_message(&conversation_id, conversation_message.clone())
            .map_err(CorelibError::ConversationStore)?;

        // Send message to DS
        println!("Sending message to DS");
        block_on(self.api_client.ds_send_message(
            params,
            &self.group_store.leaf_signing_key(group_id),
            &self.group_store.group_state_ear_key(group_id),
        ))?;
        Ok(conversation_message)
    }
}
