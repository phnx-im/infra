// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls::prelude::KeyPackage;
use phnxbackend::{
    auth_service::{credentials::ClientCredential, AsClientId, UserName},
    crypto::ear::keys::{
        AddPackageEarKey, ClientCredentialEarKey, FriendshipPackageEarKey, SignatureEarKey,
        SignatureEarKeyWrapperKey, WelcomeAttributionInfoEarKey,
    },
    messages::{client_as::FriendshipPackage, FriendshipToken},
    qs::{KeyPackageBatch, VERIFIED},
};

use uuid::Uuid;

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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
#[derive(Clone)]
pub struct PartialContact {
    pub user_name: UserName,
    // ID of the connection conversation with this contact.
    pub conversation_id: Uuid,
    pub friendship_package_ear_key: FriendshipPackageEarKey,
}

impl PartialContact {
    pub(crate) fn into_contact(
        self,
        friendship_package: FriendshipPackage,
        add_infos: Vec<ContactAddInfos>,
        client_credential: ClientCredential,
    ) -> Contact {
        Contact {
            user_name: self.user_name,
            add_infos,
            client_credentials: vec![client_credential],
            wai_ear_key: friendship_package.wai_ear_key,
            friendship_token: friendship_package.friendship_token,
            add_package_ear_key: friendship_package.add_package_ear_key,
            client_credential_ear_key: friendship_package.client_credential_ear_key,
            signature_ear_key_wrapper_key: friendship_package.signature_ear_key_wrapper_key,
            conversation_id: self.conversation_id,
        }
    }
}
