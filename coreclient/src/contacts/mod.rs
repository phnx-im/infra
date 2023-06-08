// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;

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
    client_credentials: HashMap<AsClientId, ClientCredential>,
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
        self.client_credentials.get(client_id)
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
