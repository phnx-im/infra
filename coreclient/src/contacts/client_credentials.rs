// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! # Stored client credentials
//!
//! This module provides a wrapper struct that allows the local storage of
//! client credentials.

use std::ops::{Deref, DerefMut};

use phnxtypes::credentials::ClientCredential;

pub(crate) struct StoredClientCredential {
    client_credential: ClientCredential,
}

impl Deref for StoredClientCredential {
    type Target = ClientCredential;

    fn deref(&self) -> &Self::Target {
        &self.client_credential
    }
}

impl DerefMut for StoredClientCredential {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.client_credential
    }
}

impl From<StoredClientCredential> for ClientCredential {
    fn from(stored_client_credential: StoredClientCredential) -> Self {
        stored_client_credential.client_credential
    }
}

impl From<ClientCredential> for StoredClientCredential {
    fn from(client_credential: ClientCredential) -> Self {
        Self { client_credential }
    }
}

impl StoredClientCredential {}
