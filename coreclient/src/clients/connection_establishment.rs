// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls::group::GroupId;
use payload::{ConnectionEstablishmentPackagePayload, ConnectionEstablishmentPackagePayloadIn};
use phnxtypes::{
    credentials::{
        ClientCredential, CredentialFingerprint, VerifiableClientCredential,
        keys::AsIntermediateVerifyingKey,
    },
    crypto::{
        ear::{
            EarDecryptable, EarEncryptable, GenericDeserializable, GenericSerializable,
            keys::{
                FriendshipPackageEarKey, GroupStateEarKey, IdentityLinkWrapperKey,
                WelcomeAttributionInfoEarKey,
            },
        },
        hpke::{HpkeDecryptable, HpkeEncryptable},
        indexed_aead::keys::UserProfileBaseSecret,
        kdf::keys::{ConnectionKey, ConnectionKeyType},
        signatures::{
            private_keys::SignatureVerificationError,
            signable::{Signable, Signature, SignedStruct, Verifiable, VerifiedStruct},
        },
    },
    identifiers::{Fqdn, UserId},
    messages::{
        FriendshipToken,
        client_as::{EncryptedConnectionEstablishmentPackage, EncryptedFriendshipPackageCtype},
    },
};
use tbs::{ConnectionEstablishmentPackageTbs, VerifiableConnectionEstablishmentPackage};
use tls_codec::{
    DeserializeBytes, Serialize as TlsSerializeTrait, TlsDeserializeBytes, TlsSerialize, TlsSize,
};

pub(crate) mod payload {
    use phnxtypes::{LibraryError, credentials::keys::ClientSigningKey};

    use super::*;

    #[derive(Debug, TlsDeserializeBytes, TlsSize, Clone)]
    pub(super) struct ConnectionEstablishmentPackagePayloadIn {
        pub(super) sender_client_credential: VerifiableClientCredential,
        connection_group_id: GroupId,
        connection_group_ear_key: GroupStateEarKey,
        connection_group_identity_link_wrapper_key: IdentityLinkWrapperKey,
        friendship_package_ear_key: FriendshipPackageEarKey,
        friendship_package: FriendshipPackage,
    }

    impl ConnectionEstablishmentPackagePayloadIn {
        pub(super) fn verify(
            self,
            verifying_key: &AsIntermediateVerifyingKey,
        ) -> Result<ConnectionEstablishmentPackagePayload, SignatureVerificationError> {
            let client_credential = self.sender_client_credential.verify(verifying_key)?;
            let verified_payload = ConnectionEstablishmentPackagePayload {
                sender_client_credential: client_credential,
                connection_group_id: self.connection_group_id,
                connection_group_ear_key: self.connection_group_ear_key,
                connection_group_identity_link_wrapper_key: self
                    .connection_group_identity_link_wrapper_key,
                friendship_package_ear_key: self.friendship_package_ear_key,
                friendship_package: self.friendship_package,
            };
            Ok(verified_payload)
        }
    }

    #[derive(Debug, TlsSerialize, TlsSize, Clone)]
    pub(crate) struct ConnectionEstablishmentPackagePayload {
        pub(crate) sender_client_credential: ClientCredential,
        pub(crate) connection_group_id: GroupId,
        pub(crate) connection_group_ear_key: GroupStateEarKey,
        pub(crate) connection_group_identity_link_wrapper_key: IdentityLinkWrapperKey,
        pub(crate) friendship_package_ear_key: FriendshipPackageEarKey,
        pub(crate) friendship_package: FriendshipPackage,
    }

    impl ConnectionEstablishmentPackagePayload {
        pub(crate) fn sign(
            self,
            signing_key: &ClientSigningKey,
            recipient_user_id: UserId,
        ) -> Result<ConnectionEstablishmentPackage, LibraryError> {
            let tbs =
                ConnectionEstablishmentPackageTbs::from_payload(self.clone(), recipient_user_id);
            tbs.sign(signing_key)
        }
    }
}

mod tbs {
    use super::*;
    use phnxtypes::identifiers::UserId;

    use super::payload::ConnectionEstablishmentPackagePayload;

    #[derive(Debug, TlsSerialize, TlsSize, Clone)]
    pub(super) struct ConnectionEstablishmentPackageTbs {
        payload: ConnectionEstablishmentPackagePayload,
        recipient_user_id: UserId,
    }

    impl ConnectionEstablishmentPackageTbs {
        pub(super) fn from_payload(
            payload: ConnectionEstablishmentPackagePayload,
            recipient_user_id: UserId,
        ) -> Self {
            Self {
                payload,
                recipient_user_id,
            }
        }
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

    impl SignedStruct<ConnectionEstablishmentPackageTbs> for ConnectionEstablishmentPackage {
        fn from_payload(tbs: ConnectionEstablishmentPackageTbs, signature: Signature) -> Self {
            Self {
                payload: tbs.payload,
                signature,
            }
        }
    }

    mod private_mod {
        #[derive(Default)]
        pub struct Seal;
    }

    #[derive(Debug)]
    pub(super) struct VerifiableConnectionEstablishmentPackage {
        tbs: ConnectionEstablishmentPackageTbs,
        signature: Signature,
    }

    impl VerifiableConnectionEstablishmentPackage {
        pub(super) fn from_verified_payload(
            verified_payload: ConnectionEstablishmentPackagePayload,
            recipient_user_id: UserId,
            signature: Signature,
        ) -> Self {
            let tbs = ConnectionEstablishmentPackageTbs::from_payload(
                verified_payload,
                recipient_user_id,
            );
            Self { tbs, signature }
        }

        pub(super) fn verify(
            self,
        ) -> Result<ConnectionEstablishmentPackagePayload, SignatureVerificationError> {
            let verifying_key = self
                .tbs
                .payload
                .sender_client_credential
                .verifying_key()
                .clone();
            <Self as Verifiable>::verify(self, &verifying_key)
        }
    }

    impl Verifiable for VerifiableConnectionEstablishmentPackage {
        fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
            self.tbs.tls_serialize_detached()
        }

        fn signature(&self) -> impl AsRef<[u8]> {
            &self.signature
        }

        fn label(&self) -> &str {
            "ConnectionEstablishmentPackageTBS"
        }
    }

    impl VerifiedStruct<VerifiableConnectionEstablishmentPackage>
        for ConnectionEstablishmentPackagePayload
    {
        type SealingType = private_mod::Seal;

        fn from_verifiable(
            verifiable: VerifiableConnectionEstablishmentPackage,
            _seal: Self::SealingType,
        ) -> Self {
            verifiable.tbs.payload
        }
    }
}

#[derive(Debug, TlsSerialize, TlsSize, Clone)]
pub(crate) struct ConnectionEstablishmentPackage {
    payload: ConnectionEstablishmentPackagePayload,
    signature: Signature,
}

impl GenericSerializable for ConnectionEstablishmentPackage {
    type Error = tls_codec::Error;

    fn serialize(&self) -> Result<Vec<u8>, Self::Error> {
        self.tls_serialize_detached()
    }
}

impl HpkeEncryptable<ConnectionKeyType, EncryptedConnectionEstablishmentPackage>
    for ConnectionEstablishmentPackage
{
}

#[derive(Debug, TlsDeserializeBytes, TlsSize, Clone)]
pub(super) struct ConnectionEstablishmentPackageIn {
    payload: ConnectionEstablishmentPackagePayloadIn,
    signature: Signature,
}

impl GenericDeserializable for ConnectionEstablishmentPackageIn {
    type Error = tls_codec::Error;

    fn deserialize(bytes: &[u8]) -> Result<Self, Self::Error> {
        Self::tls_deserialize_exact_bytes(bytes)
    }
}

impl ConnectionEstablishmentPackageIn {
    pub(super) fn sender_domain(&self) -> &Fqdn {
        self.payload.sender_client_credential.domain()
    }

    pub(super) fn signer_fingerprint(&self) -> &CredentialFingerprint {
        self.payload.sender_client_credential.signer_fingerprint()
    }

    pub(super) fn verify(
        self,
        verifying_key: &AsIntermediateVerifyingKey,
        recipient_user_id: UserId,
    ) -> Result<ConnectionEstablishmentPackagePayload, SignatureVerificationError> {
        let verified_payload = self.payload.verify(verifying_key)?;
        VerifiableConnectionEstablishmentPackage::from_verified_payload(
            verified_payload,
            recipient_user_id,
            self.signature,
        )
        .verify()
    }
}

impl HpkeDecryptable<ConnectionKeyType, EncryptedConnectionEstablishmentPackage>
    for ConnectionEstablishmentPackageIn
{
}

#[derive(Debug, Clone, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub(crate) struct FriendshipPackage {
    pub(crate) friendship_token: FriendshipToken,
    pub(crate) connection_key: ConnectionKey,
    pub(crate) wai_ear_key: WelcomeAttributionInfoEarKey,
    pub(crate) user_profile_base_secret: UserProfileBaseSecret,
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

impl EarEncryptable<FriendshipPackageEarKey, EncryptedFriendshipPackageCtype>
    for FriendshipPackage
{
}
impl EarDecryptable<FriendshipPackageEarKey, EncryptedFriendshipPackageCtype>
    for FriendshipPackage
{
}
