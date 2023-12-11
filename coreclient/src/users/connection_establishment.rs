// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls::group::GroupId;
use phnxtypes::{
    credentials::{keys::AsIntermediateVerifyingKey, ClientCredential, VerifiableClientCredential},
    crypto::{
        ear::{
            keys::{
                ClientCredentialEarKey, FriendshipPackageEarKey, GroupStateEarKey,
                SignatureEarKeyWrapperKey,
            },
            GenericDeserializable, GenericSerializable,
        },
        hpke::{HpkeDecryptable, HpkeEncryptable},
        signatures::signable::{Signable, Signature, SignedStruct, Verifiable, VerifiedStruct},
        ConnectionDecryptionKey, ConnectionEncryptionKey,
    },
    messages::client_as::{EncryptedConnectionEstablishmentPackage, FriendshipPackage},
};
use tls_codec::{DeserializeBytes, Serialize, TlsDeserializeBytes, TlsSerialize, TlsSize};

#[derive(Debug, TlsSerialize, TlsSize, Clone)]
pub struct ConnectionEstablishmentPackageTbs {
    pub sender_client_credential: ClientCredential,
    pub connection_group_id: GroupId,
    pub connection_group_ear_key: GroupStateEarKey,
    pub connection_group_credential_key: ClientCredentialEarKey,
    pub connection_group_signature_ear_key_wrapper_key: SignatureEarKeyWrapperKey,
    pub friendship_package_ear_key: FriendshipPackageEarKey,
    pub friendship_package: FriendshipPackage,
}

impl Signable for ConnectionEstablishmentPackageTbs {
    type SignedOutput = ConnectionEstablishmentPackage;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        "ConnectionEstablishmentPackageTBS"
    }
}

#[derive(Debug, TlsSerialize, TlsSize, Clone)]
pub struct ConnectionEstablishmentPackage {
    payload: ConnectionEstablishmentPackageTbs,
    // TBS: All information above signed by the ClientCredential.
    signature: Signature,
}

impl GenericSerializable for ConnectionEstablishmentPackage {
    type Error = tls_codec::Error;

    fn serialize(&self) -> Result<Vec<u8>, Self::Error> {
        self.tls_serialize_detached()
    }
}

impl HpkeEncryptable<ConnectionEncryptionKey, EncryptedConnectionEstablishmentPackage>
    for ConnectionEstablishmentPackage
{
}

impl SignedStruct<ConnectionEstablishmentPackageTbs> for ConnectionEstablishmentPackage {
    fn from_payload(payload: ConnectionEstablishmentPackageTbs, signature: Signature) -> Self {
        Self { payload, signature }
    }
}

mod private_mod {
    #[derive(Default)]
    pub struct Seal;
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Clone)]
pub struct ConnectionEstablishmentPackageTbsIn {
    sender_client_credential: VerifiableClientCredential,
    connection_group_id: GroupId,
    connection_group_ear_key: GroupStateEarKey,
    connection_group_credential_key: ClientCredentialEarKey,
    connection_group_signature_ear_key_wrapper_key: SignatureEarKeyWrapperKey,
    pub friendship_package_ear_key: FriendshipPackageEarKey,
    friendship_package: FriendshipPackage,
}

impl VerifiedStruct<ConnectionEstablishmentPackageIn> for ConnectionEstablishmentPackageTbsIn {
    type SealingType = private_mod::Seal;

    fn from_verifiable(
        verifiable: ConnectionEstablishmentPackageIn,
        _seal: Self::SealingType,
    ) -> Self {
        verifiable.payload
    }
}

#[derive(Debug, TlsDeserializeBytes, TlsSize, Clone)]
pub struct ConnectionEstablishmentPackageIn {
    payload: ConnectionEstablishmentPackageTbsIn,
    // TBS: All information above signed by the ClientCredential.
    signature: Signature,
}

impl GenericDeserializable for ConnectionEstablishmentPackageIn {
    type Error = tls_codec::Error;

    fn deserialize(bytes: &[u8]) -> Result<Self, Self::Error> {
        Self::tls_deserialize_exact(bytes)
    }
}

impl ConnectionEstablishmentPackageIn {
    pub fn sender_credential(&self) -> &VerifiableClientCredential {
        &self.payload.sender_client_credential
    }

    pub fn verify(
        self,
        verifying_key: &AsIntermediateVerifyingKey,
    ) -> ConnectionEstablishmentPackageTbs {
        let sender_client_credential: ClientCredential = self
            .payload
            .sender_client_credential
            .verify(verifying_key)
            .unwrap();
        ConnectionEstablishmentPackageTbs {
            sender_client_credential,
            connection_group_id: self.payload.connection_group_id,
            connection_group_ear_key: self.payload.connection_group_ear_key,
            connection_group_credential_key: self.payload.connection_group_credential_key,
            connection_group_signature_ear_key_wrapper_key: self
                .payload
                .connection_group_signature_ear_key_wrapper_key,
            friendship_package_ear_key: self.payload.friendship_package_ear_key,
            friendship_package: self.payload.friendship_package,
        }
    }
}

impl HpkeDecryptable<ConnectionDecryptionKey, EncryptedConnectionEstablishmentPackage>
    for ConnectionEstablishmentPackageIn
{
}

impl Verifiable for ConnectionEstablishmentPackageIn {
    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.payload.tls_serialize_detached()
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn label(&self) -> &str {
        "ConnectionEstablishmentPackageTBS"
    }
}
