// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls::prelude::KeyPackage;
use phnxbackend::{
    auth_service::{credentials::ClientCredential, AsClientId, UserName},
    crypto::ear::keys::WelcomeAttributionInfoEarKey,
    qs::{KeyPackageBatch, VERIFIED},
};

#[derive(Debug, Clone)]
pub struct Contact {
    username: String,
    id: UserName,
    last_resort_add_info: ContactAddInfos,
    add_infos: Vec<ContactAddInfos>,
    // These should be in the same order as the KeyPackages in the ContactInfos.
    // TODO: This is a bit brittle, but as far as I can see, there is no way to
    // otherwise correlate client credentials with KeyPackages. We might want to
    // change the signature ciphertext in the InfraCredentials to also include
    // the fingerprint of the ClientCredential s.t. we can correlate them
    // without verifying every time.
    client_credentials: Vec<ClientCredential>,
    // Encryption key for WelcomeAttributionInfos
    wai_ear_key: WelcomeAttributionInfoEarKey,
}

#[derive(Debug, Clone)]
pub(crate) struct ContactAddInfos {
    pub key_packages: Vec<KeyPackage>,
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
    pub(crate) fn add_infos(&mut self) -> ContactAddInfos {
        self.add_infos
            .pop()
            .unwrap_or(self.last_resort_add_info.clone())
    }

    pub(crate) fn wai_ear_key(&self) -> &WelcomeAttributionInfoEarKey {
        &self.wai_ear_key
    }
}
