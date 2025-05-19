// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls::{prelude::KeyPackage, versions::ProtocolVersion};
use openmls_rust_crypto::RustCrypto;
use phnxtypes::{
    LibraryError,
    credentials::pseudonymous_credentials::PseudonymousCredential,
    crypto::{
        ear::keys::{FriendshipPackageEarKey, IdentityLinkKey, WelcomeAttributionInfoEarKey},
        indexed_aead::keys::{UserProfileKey, UserProfileKeyIndex},
        kdf::keys::ConnectionKey,
    },
    identifiers::UserId,
    messages::FriendshipToken,
};
use sqlx::SqliteConnection;

use crate::{
    ConversationId,
    clients::{api_clients::ApiClients, connection_establishment::FriendshipPackage},
    groups::client_auth_info::StorableClientCredential,
    key_stores::indexed_keys::StorableIndexedKey,
};
use anyhow::Result;

pub(crate) mod persistence;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contact {
    pub client_id: UserId,
    // Encryption key for WelcomeAttributionInfos
    pub(crate) wai_ear_key: WelcomeAttributionInfoEarKey,
    pub(crate) friendship_token: FriendshipToken,
    pub(crate) connection_key: ConnectionKey,
    pub(crate) user_profile_key_index: UserProfileKeyIndex,
    // ID of the connection conversation with this contact.
    pub(crate) conversation_id: ConversationId,
}

#[derive(Debug, Clone)]
pub(crate) struct ContactAddInfos {
    pub key_package: KeyPackage,
    pub identity_link_key: IdentityLinkKey,
    pub user_profile_key: UserProfileKey,
}

impl Contact {
    pub(crate) fn from_friendship_package(
        client_id: UserId,
        conversation_id: ConversationId,
        friendship_package: FriendshipPackage,
    ) -> Result<Self, LibraryError> {
        let user_profile_key = UserProfileKey::from_base_secret(
            friendship_package.user_profile_base_secret,
            &client_id,
        )?;
        let contact = Self {
            client_id,
            wai_ear_key: friendship_package.wai_ear_key,
            friendship_token: friendship_package.friendship_token,
            connection_key: friendship_package.connection_key,
            conversation_id,
            user_profile_key_index: user_profile_key.index().clone(),
        };
        Ok(contact)
    }

    pub(crate) async fn fetch_add_infos(
        &self,
        connection: &mut SqliteConnection,
        api_clients: &ApiClients,
    ) -> Result<ContactAddInfos> {
        let invited_user_domain = self.client_id.domain();

        let key_package_response = api_clients
            .get(invited_user_domain)?
            .qs_key_package(self.friendship_token.clone())
            .await?;
        let key_package_in = key_package_response.key_package;
        // Verify the KeyPackage
        let verified_key_package =
            key_package_in.validate(&RustCrypto::default(), ProtocolVersion::default())?;
        let pseudonymous_credential = PseudonymousCredential::try_from(
            verified_key_package.leaf_node().credential().clone(),
        )?;
        // Verify the pseudonymous credential
        let (plaintext, identity_link_key) =
            pseudonymous_credential.derive_decrypt_and_verify(&self.connection_key)?;
        // Verify the client credential
        let incoming_client_credential = StorableClientCredential::verify(
            &mut *connection,
            api_clients,
            plaintext.client_credential,
        )
        .await?;
        // Check that the client credential is the same as the one we have on file.
        let Some(current_client_credential) = StorableClientCredential::load_by_client_id(
            &mut *connection,
            incoming_client_credential.identity(),
        )
        .await?
        else {
            anyhow::bail!("Client credential not found");
        };
        if current_client_credential.fingerprint() != incoming_client_credential.fingerprint() {
            anyhow::bail!("Client credential does not match");
        }
        let user_profile_key =
            UserProfileKey::load(&mut *connection, &self.user_profile_key_index).await?;
        let add_info = ContactAddInfos {
            key_package: verified_key_package,
            identity_link_key,
            user_profile_key,
        };
        Ok(add_info)
    }

    pub(crate) fn wai_ear_key(&self) -> &WelcomeAttributionInfoEarKey {
        &self.wai_ear_key
    }
}

/// Contact which has not yet accepted our connection request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartialContact {
    pub client_id: UserId,
    // ID of the connection conversation with this contact.
    pub conversation_id: ConversationId,
    pub friendship_package_ear_key: FriendshipPackageEarKey,
}

impl PartialContact {
    pub(crate) fn new(
        client_id: UserId,
        conversation_id: ConversationId,
        friendship_package_ear_key: FriendshipPackageEarKey,
    ) -> Self {
        Self {
            client_id,
            conversation_id,
            friendship_package_ear_key,
        }
    }
}
