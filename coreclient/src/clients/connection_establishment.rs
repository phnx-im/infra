// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls::group::GroupId;
use phnxtypes::{
    credentials::{keys::AsIntermediateVerifyingKey, ClientCredential, VerifiableClientCredential},
    crypto::{
        ear::{
            keys::{
                FriendshipPackageEarKey, GroupStateEarKey, IdentityLinkWrapperKey,
                KeyPackageEarKey, WelcomeAttributionInfoEarKey,
            },
            EarDecryptable, EarEncryptable, GenericDeserializable, GenericSerializable,
        },
        hpke::{HpkeDecryptable, HpkeEncryptable},
        kdf::keys::ConnectionKey,
        signatures::{
            signable::{Signable, Signature, SignedStruct, Verifiable, VerifiedStruct},
            traits::SignatureVerificationError,
        },
        ConnectionDecryptionKey, ConnectionEncryptionKey,
    },
    messages::{
        client_as::{EncryptedConnectionEstablishmentPackage, EncryptedFriendshipPackage},
        FriendshipToken,
    },
};
use tls_codec::{
    DeserializeBytes, Serialize as TlsSerializeTrait, TlsDeserializeBytes, TlsSerialize, TlsSize,
};

use crate::user_profiles::UserProfile;

#[derive(Debug, TlsSerialize, TlsSize, Clone)]
pub struct ConnectionEstablishmentPackageTbs {
    pub(crate) sender_client_credential: ClientCredential,
    pub(crate) connection_group_id: GroupId,
    pub(crate) connection_group_ear_key: GroupStateEarKey,
    pub(crate) connection_group_identity_link_wrapper_key: IdentityLinkWrapperKey,
    pub(crate) friendship_package_ear_key: FriendshipPackageEarKey,
    pub(crate) friendship_package: FriendshipPackage,
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
    connection_group_identity_link_wrapper_key: IdentityLinkWrapperKey,
    friendship_package_ear_key: FriendshipPackageEarKey,
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
        Self::tls_deserialize_exact_bytes(bytes)
    }
}

impl ConnectionEstablishmentPackageIn {
    pub fn sender_credential(&self) -> &VerifiableClientCredential {
        &self.payload.sender_client_credential
    }

    pub fn verify(
        self,
        verifying_key: &AsIntermediateVerifyingKey,
    ) -> Result<ConnectionEstablishmentPackageTbs, SignatureVerificationError> {
        let sender_client_credential: ClientCredential = self
            .payload
            .sender_client_credential
            .verify(verifying_key)?;
        Ok(ConnectionEstablishmentPackageTbs {
            sender_client_credential,
            connection_group_id: self.payload.connection_group_id,
            connection_group_ear_key: self.payload.connection_group_ear_key,
            connection_group_identity_link_wrapper_key: self
                .payload
                .connection_group_identity_link_wrapper_key,
            friendship_package_ear_key: self.payload.friendship_package_ear_key,
            friendship_package: self.payload.friendship_package,
        })
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

#[derive(Debug, Clone, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub(crate) struct FriendshipPackage {
    pub(crate) friendship_token: FriendshipToken,
    pub(crate) add_package_ear_key: KeyPackageEarKey,
    pub(crate) connection_key: ConnectionKey,
    pub(crate) wai_ear_key: WelcomeAttributionInfoEarKey,
    pub(crate) user_profile: UserProfile,
}

impl GenericSerializable for FriendshipPackage {
    type Error = tls_codec::Error;

    fn serialize(&self) -> Result<Vec<u8>, Self::Error> {
        self.tls_serialize_detached()
    }
}

impl GenericDeserializable for FriendshipPackage {
    type Error = tls_codec::Error;

    fn deserialize(bytes: &[u8]) -> Result<Self, Self::Error> {
        Self::tls_deserialize_exact_bytes(bytes)
    }
}

impl EarEncryptable<FriendshipPackageEarKey, EncryptedFriendshipPackage> for FriendshipPackage {}
impl EarDecryptable<FriendshipPackageEarKey, EncryptedFriendshipPackage> for FriendshipPackage {}
