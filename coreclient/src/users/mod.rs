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
            keys::ClientSigningKey, AsIntermediateCredential, ClientCredential,
            ClientCredentialCsr, ClientCredentialPayload,
        },
        OpaqueRegistrationRecord, OpaqueRegistrationRequest,
    },
    crypto::{
        signatures::signable::Verifiable, OpaqueCiphersuite, QueueRatchet, RatchetDecryptionKey,
    },
    messages::FriendshipToken,
    qs::AddPackage,
};
use rand::rngs::OsRng;

use super::*;

pub(crate) const CIPHERSUITE: Ciphersuite =
    Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;

pub(crate) const CONNECTION_KEYPACKAGES: usize = 50;
pub(crate) const ADD_PACKAGES: usize = 50;

pub(crate) struct MemoryUserKeyStore {
    // Client credential secret key
    signing_key: ClientSigningKey,
    // AS-specific key material
    as_queue_decryption_key: RatchetDecryptionKey,
    as_ratchet_key: QueueRatchet,
    // QS-specific key material
    qs_queue_decryption_key: RatchetDecryptionKey,
    qs_ratchet_key: QueueRatchet,
    friendship_token: FriendshipToken,
    // Leaf credentials in KeyPackages
    leaf_signers: HashMap<SignaturePublicKey, InfraCredentialSigningKey>,
}

pub struct SelfUser {
    pub(crate) crypto_backend: OpenMlsRustCrypto,
    pub(crate) api_client: ApiClient,
    pub(crate) username: String,
    pub(crate) conversation_store: ConversationStore,
    pub(crate) group_store: GroupStore,
    pub(crate) key_store: MemoryUserKeyStore,
}

impl SelfUser {
    /// Create a new user with the given name and a fresh set of credentials.
    pub fn new(username: String, password: String) -> Self {
        let crypto_backend = OpenMlsRustCrypto::default();
        // Let's turn TLS off for now.
        let api_client = ApiClient::initialize(
            "http://localhost:8000".to_string(),
            TransportEncryption::Off,
        )
        .unwrap();

        let (client_credential_csr, prelim_signing_key) =
            ClientCredentialCsr::new(username.clone().into(), SignatureScheme::ED25519).unwrap();

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
            client: Some(username.as_bytes()),
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

        let leaf_signers = HashMap::new();
        let friendship_token = FriendshipToken::random().unwrap();

        let key_store = MemoryUserKeyStore {
            signing_key,
            as_queue_decryption_key,
            as_ratchet_key,
            qs_queue_decryption_key,
            qs_ratchet_key,
            friendship_token,
            leaf_signers,
        };

        let user = Self {
            crypto_backend,
            api_client,
            username,
            conversation_store: ConversationStore::default(),
            group_store: GroupStore::default(),
            key_store,
        };

        let mut connection_key_packages = vec![];
        for _ in 0..CONNECTION_KEYPACKAGES {
            let kp = user.generate_keypackage();
            connection_key_packages.push(kp.into());
        }

        let opaque_registration_record = OpaqueRegistrationRecord {
            client_message: client_registration_finish_result.message,
        };

        block_on(
            api_client.as_finish_user_registration(
                username.into(),
                user.key_store
                    .as_queue_decryption_key
                    .encryption_key()
                    .clone(),
                as_ratchet_key,
                connection_key_packages,
                opaque_registration_record,
                &user.key_store.signing_key,
            ),
        )
        .unwrap();

        // AS registration is complete, now create the user on the QS.
        let icc_ciphertext = todo!();
        //let icc_ciphertext = signing_key.credential();
        let mut qs_add_packages = vec![];
        for _ in 0..ADD_PACKAGES {
            let kp = user.generate_keypackage();
            let add_package = AddPackage::new(
                kp.clone(),
                user.key_store.qs_ratchet_key.clone(),
                user.key_store.qs_queue_decryption_key.clone(),
            );
            qs_add_packages.push(kp.into());
        }

        block_on(api_client.qs_create_user(
            user.key_store.friendship_token.clone(),
            user.key_store.qs_ratchet_key.clone(),
            user.key_store.qs_queue_decryption_key.clone(),
            qs_add_packages,
            &user.key_store.signing_key,
        ));

        user
    }

    pub(crate) fn generate_keypackage(&self) -> KeyPackage {
        let signer = InfraCredentialSigningKey::generate(&self.key_store.signing_key);
        let credential_with_key = CredentialWithKey {
            credential: signer.credential().clone().into(),
            signature_key: signer.credential().verifying_key().clone(),
        };
        let kp = KeyPackage::builder()
            .build(
                CryptoConfig {
                    ciphersuite: CIPHERSUITE,
                    version: ProtocolVersion::Mls10,
                },
                &self.crypto_backend,
                &signer,
                credential_with_key,
            )
            .unwrap();
        self.key_store
            .leaf_signers
            .insert(signer.credential().verifying_key().clone(), signer);
        kp
    }

    /// Process received messages by group
    pub fn process_messages(
        &mut self,
        group_id: Uuid,
        messages: Vec<MlsMessageIn>,
    ) -> Result<Vec<DispatchedConversationMessage>, CorelibError> {
        let mut notification_messages = vec![];
        match self.group_store.get_group_mut(&group_id) {
            Some(group) => {
                for message in messages {
                    let processed_message = group.process_message(&self.crypto_backend, message);
                    match processed_message {
                        Ok(processed_message) => {
                            let sender_credential = processed_message.credential().clone();
                            let conversation_messages = match processed_message.into_content() {
                                ProcessedMessageContent::ApplicationMessage(
                                    application_message,
                                ) => application_message_to_conversation_messages(
                                    &sender_credential,
                                    application_message,
                                ),
                                ProcessedMessageContent::ProposalMessage(_) => {
                                    unimplemented!()
                                }
                                ProcessedMessageContent::StagedCommitMessage(staged_commit) => {
                                    staged_commit_to_conversation_messages(
                                        &sender_credential,
                                        &staged_commit,
                                    )
                                }
                                ProcessedMessageContent::ExternalJoinProposalMessage(_) => todo!(),
                            };

                            for conversation_message in conversation_messages {
                                let dispatched_conversation_message =
                                    DispatchedConversationMessage {
                                        conversation_id: UuidBytes::from_uuid(&group_id),
                                        conversation_message: conversation_message.clone(),
                                    };
                                self.conversation_store
                                    .store_message(&group_id, conversation_message)?;
                                notification_messages.push(dispatched_conversation_message);
                            }
                        }
                        Err(e) => {
                            println!("Error occured while processing inbound messages: {:?}", e);
                        }
                    }
                }

                Ok(notification_messages)
            }
            None => Err(CorelibError::GroupStore(GroupStoreError::UnknownGroup)),
        }
    }
}
