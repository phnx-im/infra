// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ops::Deref;

use openmls::{prelude::KeyPackage, versions::ProtocolVersion};
use phnxtypes::{
    credentials::ClientCredential,
    crypto::{
        ear::{
            keys::{
                AddPackageEarKey, ClientCredentialEarKey, FriendshipPackageEarKey, SignatureEarKey,
                SignatureEarKeyWrapperKey, WelcomeAttributionInfoEarKey,
            },
            EarDecryptable,
        },
        signatures::signable::Verifiable,
    },
    identifiers::{AsClientId, UserName},
    keypackage_batch::{KeyPackageBatch, VERIFIED},
    messages::FriendshipToken,
};

use crate::{
    key_stores::qs_verifying_keys::QsVerifyingKeyStore,
    users::{
        api_clients::ApiClients, openmls_provider::PhnxOpenMlsProvider, user_profile::UserProfile,
    },
    ConversationId,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};

pub(crate) mod store;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub user_name: UserName,
    // These should be in the same order as the KeyPackages in the ContactInfos.
    // TODO: This is a bit brittle, but as far as I can see, there is no way to
    // otherwise correlate client credentials with KeyPackages. We might want to
    // change the signature ciphertext in the InfraCredentials to also include
    // the fingerprint of the ClientCredential s.t. we can correlate them
    // without verifying every time.
    pub(crate) client_credentials: Vec<ClientCredential>,
    // Encryption key for WelcomeAttributionInfos
    pub(crate) wai_ear_key: WelcomeAttributionInfoEarKey,
    pub(crate) friendship_token: FriendshipToken,
    pub(crate) add_package_ear_key: AddPackageEarKey,
    pub(crate) client_credential_ear_key: ClientCredentialEarKey,
    pub(crate) signature_ear_key_wrapper_key: SignatureEarKeyWrapperKey,
    // ID of the connection conversation with this contact.
    pub(crate) conversation_id: ConversationId,
    pub(crate) user_profile: UserProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ContactAddInfos {
    pub key_packages: Vec<(KeyPackage, SignatureEarKey)>,
    pub key_package_batch: KeyPackageBatch<VERIFIED>,
}

impl Contact {
    /// Get the user name of this contact.
    pub fn user_name(&self) -> &UserName {
        &self.user_name
    }

    pub(crate) fn client_credential(&self, client_id: &AsClientId) -> Option<&ClientCredential> {
        self.client_credentials
            .iter()
            .find(|cred| &cred.identity() == client_id)
    }

    pub(crate) async fn fetch_add_infos(
        &self,
        api_clients: ApiClients,
        qs_verifying_key_store: QsVerifyingKeyStore<'_>,
        crypto_provider: &<PhnxOpenMlsProvider<'_> as openmls_traits::OpenMlsProvider>::CryptoProvider,
    ) -> Result<ContactAddInfos> {
        let invited_user = self.user_name.clone();
        let contact = self;
        let invited_user_domain = invited_user.domain();

        let key_package_batch_response = api_clients
            .get(&invited_user_domain)?
            .qs_key_package_batch(
                contact.friendship_token.clone(),
                contact.add_package_ear_key.clone(),
            )
            .await?;
        let key_packages: Vec<(KeyPackage, SignatureEarKey)> = key_package_batch_response
            .add_packages
            .into_iter()
            .map(|add_package| {
                let verified_add_package =
                    add_package.validate(crypto_provider, ProtocolVersion::default())?;
                let key_package = verified_add_package.key_package().clone();
                let sek = SignatureEarKey::decrypt(
                    &contact.signature_ear_key_wrapper_key,
                    verified_add_package.encrypted_signature_ear_key(),
                )?;
                Ok((key_package, sek))
            })
            .collect::<Result<Vec<_>>>()?;
        let qs_verifying_key = qs_verifying_key_store.get(&invited_user_domain).await?;
        let key_package_batch = key_package_batch_response
            .key_package_batch
            .verify(qs_verifying_key.deref().deref())?;
        let add_info = ContactAddInfos {
            key_package_batch,
            key_packages,
        };
        Ok(add_info)
    }

    pub(crate) fn client_credentials(&self) -> Vec<ClientCredential> {
        self.client_credentials.clone()
    }

    pub(crate) fn wai_ear_key(&self) -> &WelcomeAttributionInfoEarKey {
        &self.wai_ear_key
    }

    pub fn user_profile(&self) -> &UserProfile {
        &self.user_profile
    }
}

/// Contact which has not yet accepted our connection request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialContact {
    pub user_name: UserName,
    // ID of the connection conversation with this contact.
    pub conversation_id: ConversationId,
    pub friendship_package_ear_key: FriendshipPackageEarKey,
}

impl PartialContact {
    pub(crate) fn new(
        user_name: UserName,
        conversation_id: ConversationId,
        friendship_package_ear_key: FriendshipPackageEarKey,
    ) -> Result<Self> {
        Ok(Self {
            user_name,
            conversation_id,
            friendship_package_ear_key,
        })
    }
}
