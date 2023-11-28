// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt::{Display, Formatter};

<<<<<<< HEAD
use mls_assist::openmls_traits::types::HpkeCiphertext;
=======
use mls_assist::{
    openmls_rust_crypto::OpenMlsRustCrypto,
    openmls_traits::{random::OpenMlsRand, types::HpkeCiphertext, OpenMlsProvider},
};
use rand::{Rng, SeedableRng};
>>>>>>> main
use uuid::Uuid;

use crate::crypto::{
    ear::keys::PushTokenEarKey,
    errors::RandomnessError,
    hpke::{ClientIdDecryptionKey, ClientIdEncryptionKey, HpkeDecryptable, HpkeEncryptable},
};

use super::*;

pub const QS_CLIENT_REFERENCE_EXTENSION_TYPE: u16 = 0xff00;

#[derive(
    Clone,
    Serialize,
    Deserialize,
    TlsSerialize,
    TlsDeserializeBytes,
    TlsSize,
    PartialEq,
    Eq,
    Hash,
    Debug,
)]
pub struct Fqdn {
    // TODO: We should probably use a more restrictive type here.
    domain: Vec<u8>,
}

impl Display for Fqdn {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(&self.domain))
    }
}

impl Fqdn {
    pub fn new(domain: String) -> Self {
        Self {
            domain: domain.into_bytes(),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.domain
    }
}

impl From<&str> for Fqdn {
    fn from(domain: &str) -> Self {
        Self {
            domain: domain.as_bytes().to_vec(),
        }
    }
}

impl From<String> for Fqdn {
    fn from(domain: String) -> Self {
        domain.as_str().into()
    }
}

#[derive(Debug, Clone, PartialEq, TlsSerialize, TlsSize, TlsDeserializeBytes)]
pub struct QualifiedGroupId {
    pub group_id: [u8; 16],
    pub owning_domain: Fqdn,
}

impl std::fmt::Display for QualifiedGroupId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let uuid = Uuid::from_bytes(self.group_id);
        write!(f, "{}@{}", uuid, self.owning_domain)
    }
}

#[derive(
    Clone,
    Debug,
    TlsDeserializeBytes,
    TlsSerialize,
    TlsSize,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
)]
pub struct UserName {
    pub(crate) user_name: Vec<u8>,
    pub(crate) domain: Fqdn,
}

impl From<Vec<u8>> for UserName {
    fn from(value: Vec<u8>) -> Self {
        Self::tls_deserialize_exact(&value).unwrap()
    }
}

// TODO: This string processing is way too simplistic, but it should do for now.
impl From<&str> for UserName {
    fn from(value: &str) -> Self {
        let mut split_name = value.split('@');
        let name = split_name.next().unwrap();
        // UserNames MUST be qualified
        let domain = split_name.next().unwrap();
        assert!(split_name.next().is_none());
        let domain = domain.into();
        let user_name = name.as_bytes().to_vec();
        Self { user_name, domain }
    }
}

impl UserName {
    pub fn to_bytes(&self) -> Vec<u8> {
        self.tls_serialize_detached().unwrap()
    }

    pub fn domain(&self) -> Fqdn {
        self.domain.clone()
    }
}

impl From<String> for UserName {
    fn from(value: String) -> Self {
        value.as_str().into()
    }
}

impl From<&String> for UserName {
    fn from(value: &String) -> Self {
        value.as_str().into()
    }
}

impl std::fmt::Display for UserName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}@{}",
            String::from_utf8_lossy(&self.user_name),
            self.domain
        )
    }
}

<<<<<<< HEAD
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct AsClientId {
    pub(crate) user_name: UserName,
    pub(crate) client_id: Uuid,
}

impl TlsDeserializeBytesTrait for AsClientId {
    fn tls_deserialize(bytes: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error>
    where
        Self: Sized,
    {
        let (user_name, rest) = <UserName>::tls_deserialize(bytes.as_ref())?;
        let (client_id_bytes, rest) = <[u8; 16]>::tls_deserialize(rest)?;
        let client_id = Uuid::from_bytes(client_id_bytes);
        Ok((
            Self {
                user_name,
                client_id,
            },
            rest,
        ))
    }
}

impl TlsSerializeTrait for AsClientId {
    fn tls_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, tls_codec::Error> {
        let mut written = self.user_name.tls_serialize(writer)?;
        written += self.client_id.as_bytes().tls_serialize(writer)?;
        Ok(written)
    }
}

impl Size for AsClientId {
    fn tls_serialized_len(&self) -> usize {
        self.user_name.tls_serialized_len() + self.client_id.as_bytes().len()
=======
#[derive(
    Clone,
    Debug,
    TlsDeserializeBytes,
    TlsSerialize,
    TlsSize,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    Hash,
)]
pub struct AsClientId {
    pub(crate) user_name: UserName,
    pub(crate) client_id: Vec<u8>,
}

impl AsRef<[u8]> for AsClientId {
    fn as_ref(&self) -> &[u8] {
        &self.client_id
>>>>>>> main
    }
}

impl std::fmt::Display for AsClientId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
<<<<<<< HEAD
        let client_id_str = self.client_id.to_string();
=======
        let client_id_str = String::from_utf8_lossy(&self.client_id);
>>>>>>> main
        write!(f, "{}.{}", client_id_str, self.user_name)
    }
}

impl AsClientId {
    pub fn random(user_name: UserName) -> Result<Self, RandomnessError> {
<<<<<<< HEAD
        Ok(Self {
            user_name,
            client_id: Uuid::new_v4(),
=======
        // TODO: Use a proper rng provider.
        let mut rng = rand_chacha::ChaCha20Rng::from_entropy();
        let valid_characters = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

        let length = 16;

        // Generate a random string as client id
        let client_id: String = (0..length)
            .map(|_| {
                let index = rng.gen_range(0..valid_characters.len());
                valid_characters.chars().nth(index).unwrap_or('a')
            })
            .collect();
        Ok(Self {
            user_name,
            client_id: client_id.into_bytes(),
>>>>>>> main
        })
    }

    pub fn user_name(&self) -> UserName {
        self.user_name.clone()
    }
<<<<<<< HEAD

    pub fn client_id(&self) -> Uuid {
        self.client_id
    }
=======
>>>>>>> main
}

#[derive(
    Clone,
    Debug,
    Serialize,
    Deserialize,
    TlsSerialize,
    TlsDeserializeBytes,
    TlsSize,
    PartialEq,
    Eq,
    Hash,
)]
pub struct QsClientReference {
    pub client_homeserver_domain: Fqdn,
    pub sealed_reference: SealedClientReference,
}

#[derive(
    Debug,
    Serialize,
    Deserialize,
    Clone,
    TlsSerialize,
    TlsDeserializeBytes,
    TlsSize,
    PartialEq,
    Eq,
    Hash,
)]
pub struct SealedClientReference {
    pub(crate) ciphertext: HpkeCiphertext,
}

impl From<HpkeCiphertext> for SealedClientReference {
    fn from(value: HpkeCiphertext) -> Self {
        Self { ciphertext: value }
    }
}

impl AsRef<HpkeCiphertext> for SealedClientReference {
    fn as_ref(&self) -> &HpkeCiphertext {
        &self.ciphertext
    }
}

/// Info describing the queue configuration for a member of a given group.
#[derive(TlsSerialize, TlsDeserializeBytes, TlsSize, Serialize, Deserialize, Clone)]
pub struct ClientConfig {
    pub client_id: QsClientId,
    // Some clients might not use push tokens.
    pub push_token_ear_key: Option<PushTokenEarKey>,
}

impl HpkeEncryptable<ClientIdEncryptionKey, SealedClientReference> for ClientConfig {}
impl HpkeDecryptable<ClientIdDecryptionKey, SealedClientReference> for ClientConfig {}

/// This is the pseudonymous client id used on the QS.
<<<<<<< HEAD
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct QsClientId {
    pub(crate) client_id: Uuid,
}

impl tls_codec::Serialize for QsClientId {
    fn tls_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, tls_codec::Error> {
        self.client_id.as_bytes().tls_serialize(writer)
    }
}

impl tls_codec::DeserializeBytes for QsClientId {
    fn tls_deserialize(bytes: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error>
    where
        Self: Sized,
    {
        let (uuid_bytes, rest) = <[u8; 16]>::tls_deserialize(bytes)?;
        let client_id = Uuid::from_bytes(uuid_bytes);
        Ok((Self { client_id }, rest))
    }
}

impl tls_codec::Size for QsClientId {
    fn tls_serialized_len(&self) -> usize {
        self.client_id.as_bytes().len()
    }
}

impl QsClientId {
    pub fn random() -> Self {
        let client_id = Uuid::new_v4();
        Self { client_id }
    }

    pub fn as_uuid(&self) -> &Uuid {
=======
#[derive(
    TlsSerialize,
    TlsDeserializeBytes,
    TlsSize,
    Serialize,
    Deserialize,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Hash,
)]
pub struct QsClientId {
    pub(crate) client_id: Vec<u8>,
}

impl QsClientId {
    pub fn from_bytes(client_id: Vec<u8>) -> Self {
        Self { client_id }
    }

    pub fn random() -> Self {
        let client_id = OpenMlsRustCrypto::default().rand().random_vec(32).unwrap();
        Self { client_id }
    }

    pub fn as_slice(&self) -> &[u8] {
>>>>>>> main
        &self.client_id
    }
}

<<<<<<< HEAD
impl From<Uuid> for QsClientId {
    fn from(value: Uuid) -> Self {
        Self { client_id: value }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct QsUserId {
    pub(crate) user_id: Uuid,
}

impl From<Uuid> for QsUserId {
    fn from(value: Uuid) -> Self {
        Self { user_id: value }
    }
=======
#[derive(
    Clone,
    Debug,
    Serialize,
    Deserialize,
    TlsSerialize,
    TlsDeserializeBytes,
    TlsSize,
    PartialEq,
    Eq,
    Hash,
)]
pub struct QsUserId {
    pub(crate) user_id: Vec<u8>,
>>>>>>> main
}

impl QsUserId {
    pub fn random() -> Self {
<<<<<<< HEAD
        let user_id = Uuid::new_v4();
        Self { user_id }
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.user_id
    }
}

impl tls_codec::Serialize for QsUserId {
    fn tls_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, tls_codec::Error> {
        self.user_id.as_bytes().tls_serialize(writer)
    }
}

impl tls_codec::DeserializeBytes for QsUserId {
    fn tls_deserialize(bytes: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error>
    where
        Self: Sized,
    {
        let (uuid_bytes, rest) = <[u8; 16]>::tls_deserialize(bytes)?;
        let user_id = Uuid::from_bytes(uuid_bytes);
        Ok((Self { user_id }, rest))
    }
}

impl tls_codec::Size for QsUserId {
    fn tls_serialized_len(&self) -> usize {
        self.user_id.as_bytes().len()
    }
=======
        let user_id = OpenMlsRustCrypto::default().rand().random_vec(32).unwrap();
        Self { user_id }
    }
>>>>>>> main
}
