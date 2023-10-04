// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::openmls::prelude::GroupId;
use tls_codec::Serialize;

use crate::{
    crypto::{
        ear::{
            keys::{
                ClientCredentialEarKey, SignatureEarKeyWrapperKey, WelcomeAttributionInfoEarKey,
            },
            EarDecryptable, EarEncryptable,
        },
        signatures::signable::{Signable, Signature, SignedStruct, Verifiable, VerifiedStruct},
    },
    identifiers::AsClientId,
};

use super::*;

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Serialize, Deserialize)]
pub struct WelcomeAttributionInfoPayload {
    sender_client_id: AsClientId,
    client_credential_encryption_key: ClientCredentialEarKey,
    signature_encryption_key: SignatureEarKeyWrapperKey,
}

impl WelcomeAttributionInfoPayload {
    pub fn new(
        sender_client_id: AsClientId,
        client_credential_encryption_key: ClientCredentialEarKey,
        signature_ear_key_wrapper_key: SignatureEarKeyWrapperKey,
    ) -> Self {
        Self {
            sender_client_id,
            client_credential_encryption_key,
            signature_encryption_key: signature_ear_key_wrapper_key,
        }
    }

    pub fn client_credential_encryption_key(&self) -> &ClientCredentialEarKey {
        &self.client_credential_encryption_key
    }

    pub fn signature_ear_key_wrapper_key(&self) -> &SignatureEarKeyWrapperKey {
        &self.signature_encryption_key
    }
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct WelcomeAttributionInfoTbs {
    pub payload: WelcomeAttributionInfoPayload,
    pub group_id: GroupId,
    pub welcome: Vec<u8>,
}

impl Signable for WelcomeAttributionInfoTbs {
    type SignedOutput = WelcomeAttributionInfo;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        "WelcomeAttributionInfo"
    }
}

impl SignedStruct<WelcomeAttributionInfoTbs> for WelcomeAttributionInfo {
    fn from_payload(payload: WelcomeAttributionInfoTbs, signature: Signature) -> Self {
        Self {
            payload: payload.payload,
            signature,
        }
    }
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Serialize, Deserialize)]
pub struct WelcomeAttributionInfo {
    payload: WelcomeAttributionInfoPayload,
    signature: Signature,
}

impl WelcomeAttributionInfo {
    pub fn new(payload: WelcomeAttributionInfoPayload, signature: Signature) -> Self {
        Self { payload, signature }
    }

    pub fn into_verifiable(
        self,
        group_id: GroupId,
        serialized_welcome: Vec<u8>,
    ) -> VerifiableWelcomeAttributionInfo {
        let tbs = WelcomeAttributionInfoTbs {
            payload: self.payload,
            group_id,
            welcome: serialized_welcome,
        };
        VerifiableWelcomeAttributionInfo {
            payload: tbs,
            signature: self.signature,
        }
    }
}

#[derive(Debug)]
pub struct VerifiableWelcomeAttributionInfo {
    payload: WelcomeAttributionInfoTbs,
    signature: Signature,
}

impl VerifiableWelcomeAttributionInfo {
    pub fn sender(&self) -> AsClientId {
        self.payload.payload.sender_client_id.clone()
    }
}

impl Verifiable for VerifiableWelcomeAttributionInfo {
    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.payload.tls_serialize_detached()
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn label(&self) -> &str {
        "WelcomeAttributionInfo"
    }
}

mod private_mod {
    #[derive(Default)]
    pub struct Seal;
}

impl VerifiedStruct<VerifiableWelcomeAttributionInfo> for WelcomeAttributionInfoPayload {
    type SealingType = private_mod::Seal;

    fn from_verifiable(
        verifiable: VerifiableWelcomeAttributionInfo,
        _seal: Self::SealingType,
    ) -> Self {
        verifiable.payload.payload
    }
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Clone)]
pub struct EncryptedWelcomeAttributionInfo {
    ciphertext: Ciphertext,
}

impl AsRef<Ciphertext> for EncryptedWelcomeAttributionInfo {
    fn as_ref(&self) -> &Ciphertext {
        &self.ciphertext
    }
}

impl From<Ciphertext> for EncryptedWelcomeAttributionInfo {
    fn from(ciphertext: Ciphertext) -> Self {
        Self { ciphertext }
    }
}

impl EarEncryptable<WelcomeAttributionInfoEarKey, EncryptedWelcomeAttributionInfo>
    for WelcomeAttributionInfo
{
}

impl EarDecryptable<WelcomeAttributionInfoEarKey, EncryptedWelcomeAttributionInfo>
    for WelcomeAttributionInfo
{
}
