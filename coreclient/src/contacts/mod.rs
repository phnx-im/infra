// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ops::Deref;

use openmls::{prelude::KeyPackage, versions::ProtocolVersion};
use openmls_rust_crypto::RustCrypto;
use phnxtypes::{
    credentials::infra_credentials::PseudonymousCredential,
    crypto::{
        ear::keys::{
            FriendshipPackageEarKey, IdentityLinkKey, KeyPackageEarKey,
            WelcomeAttributionInfoEarKey,
        },
        kdf::{keys::ConnectionKey, KdfDerivable},
        signatures::signable::Verifiable,
    },
    identifiers::{AsClientId, QualifiedUserName},
    keypackage_batch::{KeyPackageBatch, VERIFIED},
    messages::FriendshipToken,
};

use crate::{
    clients::{api_clients::ApiClients, connection_establishment::FriendshipPackage},
    key_stores::qs_verifying_keys::StorableQsVerifyingKey,
    utils::persistence::SqliteConnection,
    ConversationId,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};

pub(crate) mod persistence;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub user_name: QualifiedUserName,
    pub(crate) clients: Vec<AsClientId>,
    // Encryption key for WelcomeAttributionInfos
    pub(crate) wai_ear_key: WelcomeAttributionInfoEarKey,
    pub(crate) friendship_token: FriendshipToken,
    pub(crate) add_package_ear_key: KeyPackageEarKey,
    pub(crate) connection_key: ConnectionKey,
    // ID of the connection conversation with this contact.
    pub(crate) conversation_id: ConversationId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ContactAddInfos {
    pub key_packages: Vec<(KeyPackage, IdentityLinkKey)>,
    pub key_package_batch: KeyPackageBatch<VERIFIED>,
}

impl Contact {
    pub(crate) fn from_friendship_package(
        client_id: AsClientId,
        conversation_id: ConversationId,
        friendship_package: FriendshipPackage,
    ) -> Self {
        Self {
            user_name: client_id.user_name(),
            clients: vec![client_id],
            wai_ear_key: friendship_package.wai_ear_key,
            friendship_token: friendship_package.friendship_token,
            add_package_ear_key: friendship_package.add_package_ear_key,
            connection_key: friendship_package.connection_key,
            conversation_id,
        }
    }

    /// Get the user name of this contact.
    pub fn user_name(&self) -> &QualifiedUserName {
        &self.user_name
    }

    pub(crate) async fn fetch_add_infos(
        &self,
        connection_mutex: SqliteConnection,
        api_clients: ApiClients,
    ) -> Result<ContactAddInfos> {
        let invited_user = self.user_name.clone();
        let invited_user_domain = invited_user.domain();

        let key_package_batch_response = api_clients
            .get(&invited_user_domain)?
            .qs_key_package_batch(
                self.friendship_token.clone(),
                self.add_package_ear_key.clone(),
            )
            .await?;
        let key_packages: Vec<(KeyPackage, IdentityLinkKey)> = key_package_batch_response
            .key_packages
            .into_iter()
            .map(|key_package| {
                let verified_key_package =
                    key_package.validate(&RustCrypto::default(), ProtocolVersion::default())?;
                let pseudonymous_credential = PseudonymousCredential::try_from(
                    verified_key_package.leaf_node().credential().clone(),
                )?;
                let ilk = IdentityLinkKey::derive(
                    &self.connection_key,
                    pseudonymous_credential.tbs().clone(),
                )?;
                Ok((verified_key_package, ilk))
            })
            .collect::<Result<Vec<_>>>()?;
        let qs_verifying_key =
            StorableQsVerifyingKey::get(connection_mutex, &invited_user_domain, &api_clients)
                .await?;
        let key_package_batch = key_package_batch_response
            .key_package_batch
            .verify(qs_verifying_key.deref())?;
        let add_info = ContactAddInfos {
            key_package_batch,
            key_packages,
        };
        Ok(add_info)
    }

    pub(crate) fn clients(&self) -> &[AsClientId] {
        &self.clients
    }

    pub(crate) fn wai_ear_key(&self) -> &WelcomeAttributionInfoEarKey {
        &self.wai_ear_key
    }
}

/// Contact which has not yet accepted our connection request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialContact {
    pub user_name: QualifiedUserName,
    // ID of the connection conversation with this contact.
    pub conversation_id: ConversationId,
    pub friendship_package_ear_key: FriendshipPackageEarKey,
}

impl PartialContact {
    pub(crate) fn new(
        user_name: QualifiedUserName,
        conversation_id: ConversationId,
        friendship_package_ear_key: FriendshipPackageEarKey,
    ) -> Self {
        Self {
            user_name,
            conversation_id,
            friendship_package_ear_key,
        }
    }
}
