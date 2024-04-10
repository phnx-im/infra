// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    fmt::{Display, Formatter},
    hash::Hash,
    str::FromStr,
};

use mls_assist::openmls_traits::types::HpkeCiphertext;
use url::Host;
use uuid::Uuid;

use crate::crypto::{
    ear::keys::PushTokenEarKey,
    errors::RandomnessError,
    hpke::{ClientIdDecryptionKey, ClientIdEncryptionKey, HpkeDecryptable, HpkeEncryptable},
};

use super::*;

pub const QS_CLIENT_REFERENCE_EXTENSION_TYPE: u16 = 0xff00;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct Fqdn {
    domain: Host<String>,
}

impl Size for Fqdn {
    fn tls_serialized_len(&self) -> usize {
        if let Host::Domain(domain) = &self.domain {
            domain.as_str().as_bytes().tls_serialized_len()
        } else {
            0
        }
    }
}

impl TlsDeserializeBytesTrait for Fqdn {
    fn tls_deserialize_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error>
    where
        Self: Sized,
    {
        let (domain_bytes, rest) = <Vec<u8>>::tls_deserialize_bytes(bytes)?;
        let domain_string = String::from_utf8(domain_bytes).map_err(|_| {
            tls_codec::Error::DecodingError("Couldn't decode domain string.".to_owned())
        })?;
        let domain = Fqdn::try_from(domain_string).map_err(|e| {
            let e = format!("Couldn't decode domain string: {}.", e);
            tls_codec::Error::DecodingError(e)
        })?;
        Ok((domain, rest))
    }
}

impl TlsSerializeTrait for Fqdn {
    fn tls_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, tls_codec::Error> {
        if let Host::Domain(domain) = &self.domain {
            domain.as_str().as_bytes().tls_serialize(writer)
        } else {
            Ok(0)
        }
    }
}

impl Display for Fqdn {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.domain)
    }
}

#[derive(Debug, Clone, Error)]
pub enum FqdnError {
    #[error("The given string does not represent a valid domain name.")]
    NotADomainName,
    #[error(transparent)]
    UrlError(#[from] url::ParseError),
}

impl TryFrom<String> for Fqdn {
    type Error = FqdnError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl TryFrom<&str> for Fqdn {
    type Error = FqdnError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let domain = Host::<String>::parse(&value)?;
        // Fqdns can't be IP addresses.
        if !matches!(domain, Host::Domain(_)) {
            return Err(FqdnError::NotADomainName);
        }
        Ok(Self { domain })
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

#[derive(Debug, Clone, Error)]
pub enum QualifiedGroupIdError {
    #[error(transparent)]
    FqdnError(#[from] FqdnError),
    #[error("The given string does not represent a valid qualified group id.")]
    InvalidQualifiedGroupId,
}

impl TryFrom<String> for QualifiedGroupId {
    type Error = QualifiedGroupIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl TryFrom<&str> for QualifiedGroupId {
    type Error = QualifiedGroupIdError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut split_string = value.split('@');
        let group_id = split_string.next().ok_or_else(|| {
            tracing::debug!("The given string is empty.");
            QualifiedGroupIdError::InvalidQualifiedGroupId
        })?;

        let group_id_uuid = Uuid::from_str(group_id).map_err(|_| {
            tracing::debug!("The given group id is not a valid UUID.");
            QualifiedGroupIdError::InvalidQualifiedGroupId
        })?;
        let group_id = group_id_uuid.into_bytes();
        // GroupIds MUST be qualified
        let domain = split_string.next().ok_or_else(|| {
            tracing::debug!("The given group id is not qualified.");
            QualifiedGroupIdError::InvalidQualifiedGroupId
        })?;
        let owning_domain = <Fqdn as TryFrom<&str>>::try_from(domain)?;
        if split_string.next().is_some() {
            tracing::debug!("The domain name may not contain a '@'.");
            return Err(QualifiedGroupIdError::InvalidQualifiedGroupId);
        }

        Ok(Self {
            group_id,
            owning_domain,
        })
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

#[derive(Debug, Clone, Error)]
pub enum UserNameError {
    #[error("The given string does not represent a valid user name.")]
    InvalidUserName,
    #[error(transparent)]
    FqdnError(#[from] FqdnError),
}

impl<T> SafeTryInto<T> for T {
    type Error = std::convert::Infallible;

    fn try_into(self) -> Result<T, Self::Error> {
        Ok(self)
    }
}

// Convenience trait to allow `impl TryInto<UserName>` as function input.
pub trait SafeTryInto<T>: Sized {
    type Error: std::error::Error + Send + Sync + 'static;
    fn try_into(self) -> Result<T, Self::Error>;
}

// TODO: This string processing is way too simplistic, but it should do for now.
impl SafeTryInto<UserName> for &str {
    type Error = UserNameError;

    fn try_into(self) -> Result<UserName, Self::Error> {
        let mut split_name = self.split('@');
        let name = split_name.next().ok_or(UserNameError::InvalidUserName)?;
        // UserNames MUST be qualified
        let domain = split_name.next().ok_or(UserNameError::InvalidUserName)?;
        if split_name.next().is_some() {
            return Err(UserNameError::InvalidUserName);
        }
        let domain = <Fqdn as TryFrom<&str>>::try_from(domain)?;
        let user_name = name.as_bytes().to_vec();
        Ok(UserName { user_name, domain })
    }
}

impl SafeTryInto<UserName> for String {
    type Error = UserNameError;

    fn try_into(self) -> Result<UserName, UserNameError> {
        <&str as SafeTryInto<UserName>>::try_into(self.as_str())
    }
}

impl SafeTryInto<UserName> for &String {
    type Error = UserNameError;

    fn try_into(self) -> Result<UserName, UserNameError> {
        <&str as SafeTryInto<UserName>>::try_into(self.as_str())
    }
}

impl UserName {
    pub fn domain(&self) -> Fqdn {
        self.domain.clone()
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

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct AsClientId {
    pub(crate) user_name: UserName,
    pub(crate) client_id: Uuid,
}

impl TlsDeserializeBytesTrait for AsClientId {
    fn tls_deserialize_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error>
    where
        Self: Sized,
    {
        let (user_name, rest) = <UserName>::tls_deserialize_bytes(bytes.as_ref())?;
        let (client_id_bytes, rest) = <[u8; 16]>::tls_deserialize_bytes(rest)?;
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
    }
}

impl std::fmt::Display for AsClientId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let client_id_str = self.client_id.to_string();
        write!(f, "{}.{}", client_id_str, self.user_name)
    }
}

impl AsClientId {
    pub fn random(user_name: UserName) -> Result<Self, RandomnessError> {
        Ok(Self {
            user_name,
            client_id: Uuid::new_v4(),
        })
    }

    pub fn user_name(&self) -> UserName {
        self.user_name.clone()
    }

    pub fn client_id(&self) -> Uuid {
        self.client_id
    }
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
    Debug, Serialize, Deserialize, Clone, TlsSerialize, TlsDeserializeBytes, TlsSize, PartialEq, Eq,
)]
pub struct SealedClientReference {
    pub(crate) ciphertext: HpkeCiphertext,
}

impl Hash for SealedClientReference {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.ciphertext.kem_output.hash(state);
    }
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
    fn tls_deserialize_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error>
    where
        Self: Sized,
    {
        let (uuid_bytes, rest) = <[u8; 16]>::tls_deserialize_bytes(bytes)?;
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
        &self.client_id
    }
}

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
}

impl QsUserId {
    pub fn random() -> Self {
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
    fn tls_deserialize_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error>
    where
        Self: Sized,
    {
        let (uuid_bytes, rest) = <[u8; 16]>::tls_deserialize_bytes(bytes)?;
        let user_id = Uuid::from_bytes(uuid_bytes);
        Ok((Self { user_id }, rest))
    }
}

impl tls_codec::Size for QsUserId {
    fn tls_serialized_len(&self) -> usize {
        self.user_id.as_bytes().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_fqdn() {
        let fqdn_str = "example.com";
        let fqdn = Fqdn::try_from(fqdn_str).unwrap();
        assert_eq!(fqdn.domain, Host::Domain(fqdn_str.to_string()));

        let fqdn_subdomain_str = "sub.example.com";
        let fqdn = Fqdn::try_from(fqdn_subdomain_str).unwrap();
        assert_eq!(fqdn.domain, Host::Domain(fqdn_subdomain_str.to_string()));
    }

    #[test]
    fn invalid_fqdn() {
        let fqdn_str = "invalid#domain#character";
        let result = Fqdn::try_from(fqdn_str);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FqdnError::UrlError(_)));
    }

    #[test]
    fn ip_address_fqdn() {
        let fqdn_str = "192.168.0.1";
        let result = Fqdn::try_from(fqdn_str);
        assert!(matches!(result.unwrap_err(), FqdnError::NotADomainName));
    }
}
