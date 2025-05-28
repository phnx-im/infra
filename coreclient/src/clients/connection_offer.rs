// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls::group::GroupId;
use payload::{ConnectionOfferPayload, ConnectionOfferPayloadIn};
use phnxcommon::{
    credentials::{
        ClientCredential, CredentialFingerprint, VerifiableClientCredential,
        keys::{AsIntermediateVerifyingKey, ClientSignature},
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
            signable::{Signable, SignedStruct, Verifiable, VerifiedStruct},
        },
    },
    identifiers::{Fqdn, UserId},
    messages::{
        FriendshipToken,
        client_as::{EncryptedConnectionOffer, EncryptedFriendshipPackageCtype},
    },
};
use tbs::{ConnectionOfferTbs, VerifiableConnectionOffer};
use tls_codec::{
    DeserializeBytes, Serialize as TlsSerializeTrait, TlsDeserializeBytes, TlsSerialize, TlsSize,
};

pub(crate) mod payload {
    use phnxcommon::{LibraryError, credentials::keys::ClientSigningKey};

    use super::*;

    #[derive(Debug, TlsDeserializeBytes, TlsSize, Clone)]
    pub(super) struct ConnectionOfferPayloadIn {
        pub(super) sender_client_credential: VerifiableClientCredential,
        connection_group_id: GroupId,
        connection_group_ear_key: GroupStateEarKey,
        connection_group_identity_link_wrapper_key: IdentityLinkWrapperKey,
        friendship_package_ear_key: FriendshipPackageEarKey,
        friendship_package: FriendshipPackage,
    }

    impl ConnectionOfferPayloadIn {
        pub(super) fn verify(
            self,
            verifying_key: &AsIntermediateVerifyingKey,
        ) -> Result<ConnectionOfferPayload, SignatureVerificationError> {
            let client_credential = self.sender_client_credential.verify(verifying_key)?;
            let verified_payload = ConnectionOfferPayload {
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
    #[cfg_attr(test, derive(PartialEq))]
    pub(crate) struct ConnectionOfferPayload {
        pub(crate) sender_client_credential: ClientCredential,
        pub(crate) connection_group_id: GroupId,
        pub(crate) connection_group_ear_key: GroupStateEarKey,
        pub(crate) connection_group_identity_link_wrapper_key: IdentityLinkWrapperKey,
        pub(crate) friendship_package_ear_key: FriendshipPackageEarKey,
        pub(crate) friendship_package: FriendshipPackage,
    }

    impl ConnectionOfferPayload {
        pub(crate) fn sign(
            self,
            signing_key: &ClientSigningKey,
            recipient_user_id: UserId,
        ) -> Result<ConnectionOffer, LibraryError> {
            let tbs = ConnectionOfferTbs::from_payload(self.clone(), recipient_user_id);
            tbs.sign(signing_key)
        }

        #[cfg(test)]
        pub(super) fn dummy(client_credential: ClientCredential) -> Self {
            Self {
                sender_client_credential: client_credential,
                connection_group_id: GroupId::from_slice(b"dummy_group_id"),
                connection_group_ear_key: GroupStateEarKey::random().unwrap(),
                connection_group_identity_link_wrapper_key: IdentityLinkWrapperKey::random()
                    .unwrap(),
                friendship_package_ear_key: FriendshipPackageEarKey::random().unwrap(),
                friendship_package: FriendshipPackage {
                    friendship_token: FriendshipToken::random().unwrap(),
                    connection_key: ConnectionKey::random().unwrap(),
                    wai_ear_key: WelcomeAttributionInfoEarKey::random().unwrap(),
                    user_profile_base_secret: UserProfileBaseSecret::random().unwrap(),
                },
            }
        }
    }
}

mod tbs {
    use super::*;
    use phnxcommon::{
        credentials::keys::{ClientKeyType, ClientSignature},
        identifiers::UserId,
    };

    use super::payload::ConnectionOfferPayload;

    #[derive(Debug, TlsSerialize, TlsSize, Clone)]
    pub(super) struct ConnectionOfferTbs {
        payload: ConnectionOfferPayload,
        recipient_user_id: UserId,
    }

    impl ConnectionOfferTbs {
        pub(super) fn from_payload(
            payload: ConnectionOfferPayload,
            recipient_user_id: UserId,
        ) -> Self {
            Self {
                payload,
                recipient_user_id,
            }
        }
    }

    impl Signable for ConnectionOfferTbs {
        type SignedOutput = ConnectionOffer;

        fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
            self.tls_serialize_detached()
        }

        fn label(&self) -> &str {
            "ConnectionOfferTBS"
        }
    }

    impl SignedStruct<ConnectionOfferTbs, ClientKeyType> for ConnectionOffer {
        fn from_payload(tbs: ConnectionOfferTbs, signature: ClientSignature) -> Self {
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
    pub(super) struct VerifiableConnectionOffer {
        tbs: ConnectionOfferTbs,
        signature: ClientSignature,
    }

    impl VerifiableConnectionOffer {
        pub(super) fn from_verified_payload(
            verified_payload: ConnectionOfferPayload,
            recipient_user_id: UserId,
            signature: ClientSignature,
        ) -> Self {
            let tbs = ConnectionOfferTbs::from_payload(verified_payload, recipient_user_id);
            Self { tbs, signature }
        }

        pub(super) fn verify(self) -> Result<ConnectionOfferPayload, SignatureVerificationError> {
            let verifying_key = self
                .tbs
                .payload
                .sender_client_credential
                .verifying_key()
                .clone();
            <Self as Verifiable>::verify(self, &verifying_key)
        }
    }

    impl Verifiable for VerifiableConnectionOffer {
        fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
            self.tbs.tls_serialize_detached()
        }

        fn signature(&self) -> impl AsRef<[u8]> {
            &self.signature
        }

        fn label(&self) -> &str {
            "ConnectionOfferTBS"
        }
    }

    impl VerifiedStruct<VerifiableConnectionOffer> for ConnectionOfferPayload {
        type SealingType = private_mod::Seal;

        fn from_verifiable(
            verifiable: VerifiableConnectionOffer,
            _seal: Self::SealingType,
        ) -> Self {
            verifiable.tbs.payload
        }
    }
}

#[derive(Debug, TlsSerialize, TlsSize, Clone)]
pub(crate) struct ConnectionOffer {
    payload: ConnectionOfferPayload,
    signature: ClientSignature,
}

impl GenericSerializable for ConnectionOffer {
    type Error = tls_codec::Error;

    fn serialize(&self) -> Result<Vec<u8>, Self::Error> {
        self.tls_serialize_detached()
    }
}

impl HpkeEncryptable<ConnectionKeyType, EncryptedConnectionOffer> for ConnectionOffer {}

#[derive(Debug, TlsDeserializeBytes, TlsSize, Clone)]
pub(super) struct ConnectionOfferIn {
    payload: ConnectionOfferPayloadIn,
    signature: ClientSignature,
}

impl GenericDeserializable for ConnectionOfferIn {
    type Error = tls_codec::Error;

    fn deserialize(bytes: &[u8]) -> Result<Self, Self::Error> {
        Self::tls_deserialize_exact_bytes(bytes)
    }
}

impl ConnectionOfferIn {
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
    ) -> Result<ConnectionOfferPayload, SignatureVerificationError> {
        let verified_payload = self.payload.verify(verifying_key)?;
        VerifiableConnectionOffer::from_verified_payload(
            verified_payload,
            recipient_user_id,
            self.signature,
        )
        .verify()
    }
}

impl HpkeDecryptable<ConnectionKeyType, EncryptedConnectionOffer> for ConnectionOfferIn {}

#[derive(Debug, Clone, TlsDeserializeBytes, TlsSerialize, TlsSize)]
#[cfg_attr(test, derive(PartialEq))]
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

#[cfg(test)]
mod tests {
    use phnxcommon::{
        credentials::test_utils::create_test_credentials,
        crypto::signatures::private_keys::SignatureVerificationError, identifiers::UserId,
    };
    use tls_codec::{DeserializeBytes as _, Serialize};

    use super::{ConnectionOfferIn, payload::ConnectionOfferPayload};

    #[test]
    fn signing_and_verifying() {
        let sender_user_id = UserId::random("localhost".parse().unwrap());
        let (as_sk, client_sk) = create_test_credentials(sender_user_id);
        let cep_payload = ConnectionOfferPayload::dummy(client_sk.credential().clone());
        let recipient_user_id = UserId::random("localhost".parse().unwrap());
        let cep = cep_payload
            .clone()
            .sign(&client_sk, recipient_user_id.clone())
            .unwrap();
        let cep_in =
            ConnectionOfferIn::tls_deserialize_exact_bytes(&cep.tls_serialize_detached().unwrap())
                .unwrap();
        let cep_verified = cep_in
            .clone()
            .verify(as_sk.verifying_key(), recipient_user_id)
            .unwrap();
        assert_eq!(cep_verified, cep_payload);

        // Try with a different recipient
        let recipient_user_id_2 = UserId::random("localhost".parse().unwrap());
        let err = cep_in
            .verify(as_sk.verifying_key(), recipient_user_id_2)
            .unwrap_err();
        assert!(matches!(
            err,
            SignatureVerificationError::VerificationFailure
        ));
    }
}
