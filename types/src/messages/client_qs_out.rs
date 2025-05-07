// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::openmls::prelude::KeyPackage;
use tls_codec::{TlsSerialize, TlsSize};

use crate::{
    crypto::{
        RatchetEncryptionKey, kdf::keys::RatchetSecret, signatures::keys::QsClientVerifyingKey,
    },
    identifiers::{QsClientId, QsUserId},
};

use super::push_token::EncryptedPushToken;

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct CreateClientRecordParamsOut {
    pub sender: QsUserId,
    pub client_record_auth_key: QsClientVerifyingKey,
    pub queue_encryption_key: RatchetEncryptionKey,
    pub encrypted_push_token: Option<EncryptedPushToken>,
    pub initial_ratchet_secret: RatchetSecret, // TODO: This can be dropped once we support PCS
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct PublishKeyPackagesParamsOut {
    pub sender: QsClientId,
    pub key_packages: Vec<KeyPackage>,
}
