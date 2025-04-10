// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ops::Deref;

use anyhow::Result;
use leaf_keys::LeafKeys;
use openmls::prelude::{
    CredentialWithKey, Extension, Extensions, KeyPackage, LastResortExtension, SignaturePublicKey,
    UnknownExtension,
};
use phnxtypes::{
    crypto::{
        hpke::{ClientIdEncryptionKey, HpkeEncryptable},
        kdf::keys::ConnectionKey,
    },
    identifiers::{ClientConfig, QS_CLIENT_REFERENCE_EXTENSION_TYPE, QsClientId, QsReference},
};
use sqlx::SqlitePool;
use tls_codec::Serialize as TlsSerializeTrait;

use crate::{
    clients::{CIPHERSUITE, api_clients::ApiClients},
    groups::{default_capabilities, openmls_provider::PhnxOpenMlsProvider},
};

use phnxtypes::{
    credentials::keys::ClientSigningKey,
    crypto::{
        ConnectionDecryptionKey, RatchetDecryptionKey,
        ear::keys::{KeyPackageEarKey, PushTokenEarKey, WelcomeAttributionInfoEarKey},
        signatures::keys::{QsClientSigningKey, QsUserSigningKey},
    },
    messages::FriendshipToken,
};
use serde::{Deserialize, Serialize};

pub(crate) mod as_credentials;
pub(crate) mod indexed_keys;
pub(crate) mod leaf_keys;
pub(crate) mod queue_ratchets;

// For now we persist the key store along with the user. Any key material that gets rotated in the future needs to be persisted separately.
#[derive(Clone, Serialize, Deserialize)]
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
    pub(super) key_package_ear_key: KeyPackageEarKey,
    pub(super) connection_key: ConnectionKey,
    pub(super) wai_ear_key: WelcomeAttributionInfoEarKey,
}

impl MemoryUserKeyStore {
    pub(crate) fn create_own_client_reference(&self, qs_client_id: &QsClientId) -> QsReference {
        let sealed_reference = ClientConfig {
            client_id: *qs_client_id,
            push_token_ear_key: Some(self.push_token_ear_key.clone()),
        }
        .encrypt(&self.qs_client_id_encryption_key, &[], &[]);
        QsReference {
            client_homeserver_domain: self
                .signing_key
                .credential()
                .identity()
                .user_name()
                .domain()
                .clone(),
            sealed_reference,
        }
    }

    pub(crate) async fn generate_key_package(
        &self,
        pool: &SqlitePool,
        qs_client_id: &QsClientId,
        last_resort: bool,
    ) -> Result<KeyPackage> {
        let leaf_keys = LeafKeys::generate(&self.signing_key, &self.connection_key)?;
        leaf_keys.store(pool).await?;
        let credential_with_key = leaf_keys.credential()?;
        let capabilities = default_capabilities();
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

        let mut connection = pool.acquire().await?;
        let provider = PhnxOpenMlsProvider::new(&mut connection);
        let kp = KeyPackage::builder()
            .key_package_extensions(key_package_extensions)
            .leaf_node_capabilities(capabilities)
            .leaf_node_extensions(leaf_node_extensions)
            .build(
                CIPHERSUITE,
                &provider,
                &leaf_keys.into_leaf_signer(),
                credential_with_key,
            )?;
        Ok(kp.key_package().clone())
    }
}
