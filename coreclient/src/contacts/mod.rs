// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls::prelude::KeyPackage;
use phnx_types::{
    credentials::ClientCredential,
    crypto::ear::keys::{
        AddPackageEarKey, ClientCredentialEarKey, FriendshipPackageEarKey, SignatureEarKey,
        SignatureEarKeyWrapperKey, WelcomeAttributionInfoEarKey,
    },
    identifiers::{AsClientId, UserName},
    keypackage_batch::{KeyPackageBatch, VERIFIED},
    messages::{client_as::FriendshipPackage, FriendshipToken},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub(crate) mod store;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub user_name: UserName,
    pub(crate) add_infos: Vec<ContactAddInfos>,
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
    pub(crate) conversation_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ContactAddInfos {
    pub key_packages: Vec<(KeyPackage, SignatureEarKey)>,
    pub key_package_batch: KeyPackageBatch<VERIFIED>,
}

impl Contact {
    pub(crate) fn client_credential(&self, client_id: &AsClientId) -> Option<&ClientCredential> {
        self.client_credentials
            .iter()
            .find(|cred| &cred.identity() == client_id)
    }

    pub(crate) fn client_credentials(&self) -> Vec<ClientCredential> {
        self.client_credentials.clone()
    }

    // TODO: This might be a bit wasteful, since it always removes an add_info,
    // even though the resulting commit might not succeed.
    pub(crate) fn add_infos(&mut self) -> Option<ContactAddInfos> {
        self.add_infos.pop()
    }

    pub(crate) fn wai_ear_key(&self) -> &WelcomeAttributionInfoEarKey {
        &self.wai_ear_key
    }
}

/// Contact which has not yet accepted our connection request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialContact {
    pub user_name: UserName,
    // ID of the connection conversation with this contact.
    pub conversation_id: Uuid,
    pub friendship_package_ear_key: FriendshipPackageEarKey,
}

impl PartialContact {
    pub(crate) fn new(
        user_name: UserName,
        conversation_id: Uuid,
        friendship_package_ear_key: FriendshipPackageEarKey,
    ) -> Result<Self> {
        Ok(Self {
            user_name,
            conversation_id,
            friendship_package_ear_key,
        })
    }
}
