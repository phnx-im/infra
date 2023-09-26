// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ops::Deref;

use anyhow::Result;
use openmls::{
    prelude::{
        Capabilities, CredentialWithKey, CryptoConfig, Extension, Extensions, KeyPackage,
        LastResortExtension, SignaturePublicKey, UnknownExtension,
    },
    versions::ProtocolVersion,
};
use phnxbackend::{
    crypto::{
        ear::{EarEncryptable, EncryptionError},
        hpke::HpkeEncryptable,
    },
    ds::{api::QS_CLIENT_REFERENCE_EXTENSION_TYPE, group_state::EncryptedClientCredential},
    messages::{client_as::AsQueueMessagePayload, client_ds::QsQueueMessagePayload},
    qs::{AddPackage, ClientConfig, QsClientId, QsClientReference},
};
use tls_codec::Serialize as TlsSerializeTrait;

use crate::{
    groups::{
        SUPPORTED_CIPHERSUITES, SUPPORTED_CREDENTIALS, SUPPORTED_EXTENSIONS, SUPPORTED_PROPOSALS,
        SUPPORTED_PROTOCOL_VERSIONS,
    },
    users::{api_clients::ApiClients, openmls_provider::PhnxOpenMlsProvider, CIPHERSUITE},
    utils::persistence::{DataType, Persistable, PersistenceError},
};

use anyhow::anyhow;
use phnxbackend::{
    auth_service::credentials::keys::{ClientSigningKey, InfraCredentialSigningKey},
    crypto::{
        ear::keys::{
            AddPackageEarKey, ClientCredentialEarKey, PushTokenEarKey, SignatureEarKey,
            SignatureEarKeyWrapperKey, WelcomeAttributionInfoEarKey,
        },
        kdf::keys::RatchetSecret,
        signatures::keys::{QsClientSigningKey, QsUserSigningKey},
        ConnectionDecryptionKey, RatchetDecryptionKey,
    },
    messages::{
        client_as::AsQueueRatchet, client_ds::QsQueueRatchet, FriendshipToken, QueueMessage,
    },
    qs::{ClientIdEncryptionKey, Fqdn, QsVerifyingKey},
};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use self::leaf_keys::LeafKeyStore;

pub(crate) mod as_credentials;
pub(crate) mod leaf_keys;
pub(crate) mod qs_verifying_keys;
pub(crate) mod queue_ratchets;

// For now we persist the key store along with the user. Any key material that gets rotated in the future needs to be persisted separately.
#[derive(Serialize, Deserialize)]
pub(crate) struct MemoryUserKeyStore {
    // Client credential secret key
    pub(super) signing_key: ClientSigningKey,
    // AS-specific key material
    pub(super) as_queue_decryption_key: RatchetDecryptionKey,
    pub(super) connection_decryption_key: ConnectionDecryptionKey,
    // QS-specific key material
    pub(super) qs_client_signing_key: QsClientSigningKey,
    pub(super) qs_user_signing_key: QsUserSigningKey,
    pub(super) qs_queue_decryption_key: RatchetDecryptionKey,
    pub(super) qs_client_id_encryption_key: ClientIdEncryptionKey,
    pub(super) push_token_ear_key: PushTokenEarKey,
    // These are keys that we send to our contacts
    pub(super) friendship_token: FriendshipToken,
    pub(super) add_package_ear_key: AddPackageEarKey,
    pub(super) client_credential_ear_key: ClientCredentialEarKey,
    pub(super) signature_ear_key_wrapper_key: SignatureEarKeyWrapperKey,
    pub(super) wai_ear_key: WelcomeAttributionInfoEarKey,
}

impl MemoryUserKeyStore {
    pub(crate) fn encrypt_client_credential(
        &self,
    ) -> Result<EncryptedClientCredential, EncryptionError> {
        self.signing_key
            .credential()
            .encrypt(&self.client_credential_ear_key)
    }

    pub(crate) fn create_own_client_reference(
        &self,
        qs_client_id: &QsClientId,
    ) -> QsClientReference {
        let sealed_reference = ClientConfig {
            client_id: qs_client_id.clone(),
            push_token_ear_key: Some(self.push_token_ear_key.clone()),
        }
        .encrypt(&self.qs_client_id_encryption_key, &[], &[]);
        QsClientReference {
            client_homeserver_domain: self
                .signing_key
                .credential()
                .identity_ref()
                .user_name()
                .domain(),
            sealed_reference,
        }
    }

    pub(crate) fn generate_add_package(
        &self,
        leaf_key_store: &LeafKeyStore<'_>,
        crypto_backend: &PhnxOpenMlsProvider,
        qs_client_id: &QsClientId,
        encrypted_client_credential: &EncryptedClientCredential,
        last_resort: bool,
    ) -> Result<AddPackage> {
        let leaf_keys = leaf_key_store.generate(&self.signing_key)?;
        let credential_with_key = CredentialWithKey {
            credential: leaf_keys.leaf_signing_key().credential().clone().into(),
            signature_key: leaf_keys
                .leaf_signing_key()
                .credential()
                .verifying_key()
                .clone(),
        };
        let capabilities = Capabilities::new(
            Some(&SUPPORTED_PROTOCOL_VERSIONS),
            Some(&SUPPORTED_CIPHERSUITES),
            Some(&SUPPORTED_EXTENSIONS),
            Some(&SUPPORTED_PROPOSALS),
            Some(&SUPPORTED_CREDENTIALS),
        );
        let client_reference = self.create_own_client_reference(qs_client_id);
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
                crypto_backend,
                leaf_keys.leaf_signing_key(),
                credential_with_key,
            )?;
        let esek = leaf_keys
            .signature_ear_key()
            .encrypt(&self.signature_ear_key_wrapper_key)?;

        let add_package = AddPackage::new(kp.clone(), esek, encrypted_client_credential.clone());
        Ok(add_package)
    }
}
