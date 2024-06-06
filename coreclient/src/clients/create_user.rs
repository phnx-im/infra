// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::openmls::prelude::tls_codec::*;
use opaque_ke::{RegistrationRequest, RegistrationResponse};
use phnxtypes::{
    credentials::{
        AsIntermediateCredential, PreliminaryClientSigningKey, VerifiableClientCredential,
    },
    crypto::{
        hpke::ClientIdEncryptionKey,
        opaque::{OpaqueRegistrationRecord, OpaqueRegistrationRequest},
        signatures::signable::Verifiable,
    },
    messages::{client_as::ConnectionPackage, client_qs::CreateUserRecordResponse},
    time::ExpirationData,
};
use rand_chacha::rand_core::OsRng;

use self::groups::client_auth_info::StorableClientCredential;

use super::{openmls_provider::PersistableSeed, *};

// State before any network queries
#[derive(Serialize, Deserialize)]
pub(super) struct BasicUserData {
    pub(super) as_client_id: AsClientId,
    pub(super) server_url: String,
    pub(super) password: String,
}

impl BasicUserData {
    pub(super) fn client_id(&self) -> &AsClientId {
        &self.as_client_id
    }

    pub(super) fn server_url(&self) -> &str {
        &self.server_url
    }

    pub(super) async fn prepare_as_registration(
        self,
        connection: &Connection,
        api_clients: &ApiClients,
    ) -> Result<InitialUserState> {
        // Prepare user account creation
        log::debug!("Creating new client {}", self.as_client_id);
        // Let's turn TLS off for now.
        let domain = self.as_client_id.user_name().domain();
        // Fetch credentials from AS
        let as_credential_store = AsCredentialStore::new(&connection, api_clients.clone());
        let as_intermediate_credential = as_credential_store
            .get_intermediate_credential(&domain)
            .await?;

        // We already fetch the QS encryption key here, so we don't have to do
        // it in a later step, where we otherwise don't have to perform network
        // queries and failing here means we can just start from the beginning
        // next time without incurring any costs.
        let qs_encryption_key = api_clients
            .default_client()?
            .qs_encryption_key()
            .await?
            .encryption_key;

        // Create CSR for AS to sign
        let (client_credential_csr, prelim_signing_key) =
            ClientCredentialCsr::new(self.as_client_id.clone(), SignatureScheme::ED25519)?;

        let client_credential_payload = ClientCredentialPayload::new(
            client_credential_csr,
            None,
            as_intermediate_credential.fingerprint().clone(),
        );

        // Let's do OPAQUE registration.
        // First get the server setup information.
        let mut client_rng = OsRng;
        let client_registration_start_result: ClientRegistrationStartResult<OpaqueCiphersuite> =
            ClientRegistration::<OpaqueCiphersuite>::start(
                &mut client_rng,
                self.password.as_bytes(),
            )
            .map_err(|e| anyhow!("Error starting OPAQUE handshake: {:?}", e))?;

        let initial_user_state = InitialUserState {
            client_credential_payload: client_credential_payload.clone(),
            prelim_signing_key: prelim_signing_key.clone(),
            opaque_message: client_registration_start_result
                .message
                .serialize()
                .to_vec(),
            opaque_state: client_registration_start_result.state.serialize().to_vec(),
            server_url: self.server_url,
            password: self.password,
            qs_encryption_key,
            as_intermediate_credential: as_intermediate_credential.into_credential(),
        };

        Ok(initial_user_state)
    }
}

// State pre-AS Registration
#[derive(Serialize, Deserialize)]
pub(super) struct InitialUserState {
    client_credential_payload: ClientCredentialPayload,
    prelim_signing_key: PreliminaryClientSigningKey,
    opaque_message: Vec<u8>,
    opaque_state: Vec<u8>,
    server_url: String,
    password: String,
    qs_encryption_key: ClientIdEncryptionKey,
    as_intermediate_credential: AsIntermediateCredential,
}

impl InitialUserState {
    pub(super) async fn initiate_as_registration(
        self,
        api_clients: &ApiClients,
    ) -> Result<PostRegistrationInitState> {
        let client_message =
            RegistrationRequest::<OpaqueCiphersuite>::deserialize(&self.opaque_message)
                .map_err(|e| anyhow!("Error deserializing OPAQUE message: {:?}", e))?;

        let opaque_registration_request = OpaqueRegistrationRequest { client_message };

        // Register the user with the backend.
        let response = api_clients
            .default_client()?
            .as_initiate_create_user(
                self.client_credential_payload.clone(),
                opaque_registration_request,
            )
            .await?;

        let post_registration_init_state = PostRegistrationInitState {
            initial_user_state: self,
            opaque_server_response: response
                .opaque_registration_response
                .server_message
                .serialize()
                .to_vec(),
            client_credential: response.client_credential,
        };

        Ok(post_registration_init_state)
    }

    pub(super) fn client_id(&self) -> &AsClientId {
        self.client_credential_payload.identity_ref()
    }

    pub(super) fn server_url(&self) -> &str {
        &self.server_url
    }
}

// State after server response to OPAKE initialization
#[derive(Serialize, Deserialize)]
pub(super) struct PostRegistrationInitState {
    initial_user_state: InitialUserState,
    client_credential: VerifiableClientCredential,
    opaque_server_response: Vec<u8>,
}

impl PostRegistrationInitState {
    pub(super) fn process_server_response(
        self,
        connection: &Connection,
    ) -> Result<UnfinalizedRegistrationState> {
        let InitialUserState {
            client_credential_payload,
            prelim_signing_key,
            opaque_message: _,
            opaque_state,
            server_url,
            password,
            qs_encryption_key,
            as_intermediate_credential,
        } = self.initial_user_state;

        let user_name = client_credential_payload.identity().user_name();
        let domain = user_name.domain();

        // Complete the OPAQUE registration.
        let user_name_bytes = user_name.tls_serialize_detached()?;
        let domain_bytes = domain.tls_serialize_detached()?;
        let identifiers = Identifiers {
            client: Some(&user_name_bytes),
            server: Some(&domain_bytes),
        };
        let opaque_server_response =
            RegistrationResponse::<OpaqueCiphersuite>::deserialize(&self.opaque_server_response)
                .map_err(|e| anyhow!("Error deserializing OPAQUE response: {:?}", e))?;
        let response_parameters = ClientRegistrationFinishParameters::new(identifiers, None);
        let opaque_state = ClientRegistration::<OpaqueCiphersuite>::deserialize(&opaque_state)
            .map_err(|e| anyhow!("Error deserializing OPAQUE state: {:?}", e))?;
        let mut client_rng = OsRng;
        let client_registration_finish_result: ClientRegistrationFinishResult<OpaqueCiphersuite> =
            opaque_state
                .finish(
                    &mut client_rng,
                    password.as_bytes(),
                    opaque_server_response,
                    response_parameters,
                )
                .map_err(|e| anyhow!("Error finishing OPAQUE handshake: {:?}", e))?;

        let client_credential: ClientCredential = self
            .client_credential
            .verify(as_intermediate_credential.verifying_key())?;
        StorableClientCredential::new(client_credential.clone()).store(connection)?;

        let signing_key =
            ClientSigningKey::from_prelim_key(prelim_signing_key, client_credential.clone())?;

        // Store the own client credential in the DB
        StorableClientCredential::new(client_credential.clone()).store(connection)?;

        let as_queue_decryption_key = RatchetDecryptionKey::generate()?;
        let as_initial_ratchet_secret = RatchetSecret::random()?;
        let queue_ratchet_store = QueueRatchetStore::from(connection);
        // The queue ratchets are persisted in the store. There's nothing else
        // we want to do with them here.
        queue_ratchet_store.initialize_as_queue_ratchet(as_initial_ratchet_secret.clone())?;
        let qs_initial_ratchet_secret = RatchetSecret::random()?;
        queue_ratchet_store.initialize_qs_queue_ratchet(qs_initial_ratchet_secret.clone())?;
        let qs_queue_decryption_key = RatchetDecryptionKey::generate()?;
        let qs_client_signing_key = QsClientSigningKey::random()?;
        let qs_user_signing_key = QsUserSigningKey::random()?;

        // TODO: The following five keys should be derived from a single
        // friendship key. Once that's done, remove the random constructors.
        let friendship_token = FriendshipToken::random()?;
        let add_package_ear_key = AddPackageEarKey::random()?;
        let client_credential_ear_key = ClientCredentialEarKey::random()?;
        let signature_ear_key_wrapper_key = SignatureEarKeyWrapperKey::random()?;
        let wai_ear_key: WelcomeAttributionInfoEarKey = WelcomeAttributionInfoEarKey::random()?;
        let push_token_ear_key = PushTokenEarKey::random()?;

        let connection_decryption_key = ConnectionDecryptionKey::generate()?;

        let key_store = MemoryUserKeyStore {
            signing_key,
            as_queue_decryption_key,
            connection_decryption_key,
            qs_client_signing_key,
            qs_user_signing_key,
            qs_queue_decryption_key,
            push_token_ear_key,
            friendship_token,
            add_package_ear_key,
            client_credential_ear_key,
            signature_ear_key_wrapper_key,
            wai_ear_key,
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

        let unfinalized_registration_state = UnfinalizedRegistrationState {
            key_store,
            opaque_client_message: client_registration_finish_result
                .message
                .serialize()
                .to_vec(),
            server_url,
            as_initial_ratchet_secret,
            qs_initial_ratchet_secret,
            connection_packages,
        };

        Ok(unfinalized_registration_state)
    }

    pub(super) fn client_id(&self) -> &AsClientId {
        self.client_credential.client_id()
    }

    pub(super) fn server_url(&self) -> &str {
        &self.initial_user_state.server_url
    }
}

// State after server response to OPAKE initialization
#[derive(Serialize, Deserialize)]
pub(super) struct UnfinalizedRegistrationState {
    key_store: MemoryUserKeyStore,
    opaque_client_message: Vec<u8>,
    server_url: String,
    as_initial_ratchet_secret: RatchetSecret,
    qs_initial_ratchet_secret: RatchetSecret,
    connection_packages: Vec<ConnectionPackage>,
}

impl UnfinalizedRegistrationState {
    pub(super) async fn finalize_as_registration(
        self,
        api_clients: &ApiClients,
    ) -> Result<AsRegisteredUserState> {
        let UnfinalizedRegistrationState {
            key_store,
            opaque_client_message,
            server_url,
            as_initial_ratchet_secret,
            qs_initial_ratchet_secret,
            connection_packages,
        } = self;

        let opaque_registration_record = OpaqueRegistrationRecord {
            client_message: RegistrationUpload::<OpaqueCiphersuite>::deserialize(
                &opaque_client_message,
            )
            .map_err(|e| anyhow!("Error deserializing opaque client message: {:?}", e))?,
        };

        api_clients
            .default_client()?
            .as_finish_user_registration(
                key_store.as_queue_decryption_key.encryption_key(),
                as_initial_ratchet_secret,
                connection_packages,
                opaque_registration_record,
                &key_store.signing_key,
            )
            .await?;
        let as_registered_user_state = AsRegisteredUserState {
            key_store,
            server_url,
            qs_initial_ratchet_secret,
        };
        Ok(as_registered_user_state)
    }

    pub(super) fn client_id(&self) -> &AsClientId {
        self.key_store.signing_key.credential().identity_ref()
    }

    pub(super) fn server_url(&self) -> &str {
        &self.server_url
    }
}

// State after querying finish user registration
#[derive(Serialize, Deserialize)]
pub(super) struct AsRegisteredUserState {
    key_store: MemoryUserKeyStore,
    server_url: String,
    qs_initial_ratchet_secret: RatchetSecret,
}

impl AsRegisteredUserState {
    pub(super) async fn register_with_qs(
        self,
        api_clients: &ApiClients,
    ) -> Result<QsRegisteredUserState> {
        let AsRegisteredUserState {
            key_store,
            server_url,
            qs_initial_ratchet_secret,
        } = self;

        let CreateUserRecordResponse { user_id, client_id } = api_clients
            .default_client()?
            .qs_create_user(
                key_store.friendship_token.clone(),
                key_store.qs_client_signing_key.verifying_key().clone(),
                key_store.qs_queue_decryption_key.encryption_key(),
                None,
                qs_initial_ratchet_secret,
                &key_store.qs_user_signing_key,
            )
            .await?;

        let qs_registered_user_state = QsRegisteredUserState {
            key_store,
            server_url,
            qs_user_id: user_id,
            qs_client_id: client_id,
        };

        Ok(qs_registered_user_state)
    }

    pub(super) fn client_id(&self) -> &AsClientId {
        self.key_store.signing_key.credential().identity_ref()
    }

    pub(super) fn server_url(&self) -> &str {
        &self.server_url
    }
}

// State after creating QS user
#[derive(Serialize, Deserialize)]
pub(super) struct QsRegisteredUserState {
    key_store: MemoryUserKeyStore,
    server_url: String,
    qs_user_id: QsUserId,
    qs_client_id: QsClientId,
}

impl QsRegisteredUserState {
    pub(super) async fn upload_add_packages(
        self,
        connection: &Connection,
        api_clients: &ApiClients,
    ) -> Result<PersistedUserState> {
        let QsRegisteredUserState {
            ref key_store,
            server_url: _,
            qs_user_id: _,
            ref qs_client_id,
        } = self;

        let encrypted_client_credential = key_store.encrypt_client_credential()?;
        PersistableSeed::new_random(connection)?;
        let crypto_backend = PhnxOpenMlsProvider::new(connection);

        let mut qs_add_packages = vec![];
        let leaf_key_store = LeafKeyStore::from(connection);
        for _ in 0..ADD_PACKAGES {
            // TODO: Which key do we need to use for encryption here? Probably
            // the client credential ear key, since friends need to be able to
            // decrypt it. We might want to use a separate key, though.
            let add_package = key_store.generate_add_package(
                &leaf_key_store,
                &crypto_backend,
                &qs_client_id,
                &encrypted_client_credential,
                false,
            )?;
            qs_add_packages.push(add_package);
        }
        let last_resort_add_package = key_store.generate_add_package(
            &leaf_key_store,
            &crypto_backend,
            &qs_client_id,
            &encrypted_client_credential,
            true,
        )?;
        qs_add_packages.push(last_resort_add_package);

        // Upload add packages
        api_clients
            .default_client()?
            .qs_publish_key_packages(
                qs_client_id.clone(),
                qs_add_packages,
                key_store.add_package_ear_key.clone(),
                &key_store.qs_client_signing_key,
            )
            .await?;

        let state = PersistedUserState { state: self };

        Ok(state)
    }

    pub(super) fn client_id(&self) -> &AsClientId {
        self.key_store.signing_key.credential().identity_ref()
    }

    pub(super) fn server_url(&self) -> &str {
        &self.server_url
    }
}

// State after creating QS user
#[derive(Serialize, Deserialize)]
pub(super) struct PersistedUserState {
    state: QsRegisteredUserState,
}

impl PersistedUserState {
    pub(super) fn into_self_user(
        self,
        connection: Connection,
        api_clients: ApiClients,
    ) -> SelfUser {
        let QsRegisteredUserState {
            key_store,
            server_url: _,
            qs_user_id,
            qs_client_id,
        } = self.state;
        SelfUser {
            sqlite_connection: connection,
            key_store,
            _qs_user_id: qs_user_id,
            qs_client_id: qs_client_id,
            api_clients: api_clients.clone(),
        }
    }

    pub(super) fn client_id(&self) -> &AsClientId {
        self.state.key_store.signing_key.credential().identity_ref()
    }

    pub(super) fn server_url(&self) -> &str {
        &self.state.server_url
    }

    pub(super) fn qs_user_id(&self) -> &QsUserId {
        &self.state.qs_user_id
    }

    pub(super) fn qs_client_id(&self) -> &QsClientId {
        &self.state.qs_client_id
    }
}
