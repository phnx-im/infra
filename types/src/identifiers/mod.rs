// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{fmt, hash::Hash, str::FromStr};

use mls_assist::{openmls::group::GroupId, openmls_traits::types::HpkeCiphertext};
use rand::{CryptoRng, Rng, RngCore};
use sqlx::{
    Database, Decode, Encode, Postgres, Sqlite, Type, encode::IsNull, error::BoxDynError,
    postgres::PgValueRef,
};
use tls_codec_impls::TlsUuid;
use tracing::{debug, error};
use url::Host;
use uuid::Uuid;

use crate::crypto::{
    ear::keys::PushTokenEarKey,
    errors::RandomnessError,
    hpke::{ClientIdKeyType, HpkeDecryptable, HpkeEncryptable},
};

use super::*;

mod tls_codec_impls;

pub use tls_codec_impls::TlsString;

pub const QS_CLIENT_REFERENCE_EXTENSION_TYPE: u16 = 0xff00;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Fqdn {
    domain: Host<String>,
}

impl<DB: Database> Type<DB> for Fqdn
where
    String: Type<DB>,
{
    fn type_info() -> DB::TypeInfo {
        <String as Type<DB>>::type_info()
    }
}

impl<'r, DB: Database> Encode<'r, DB> for Fqdn
where
    String: Encode<'r, DB>,
{
    fn encode_by_ref(
        &self,
        buf: &mut <DB as Database>::ArgumentBuffer<'r>,
    ) -> Result<IsNull, BoxDynError> {
        Encode::<DB>::encode(self.to_string(), buf)
    }
}

impl<'r> Decode<'r, Postgres> for Fqdn {
    fn decode(value: PgValueRef<'r>) -> Result<Self, BoxDynError> {
        let s: &str = Decode::<Postgres>::decode(value)?;
        let fqdn = s.parse().map_err(|error| {
            error!(%error, "Error parsing Fqdn from DB");
            sqlx::Error::Decode(Box::new(error))
        })?;
        Ok(fqdn)
    }
}

impl fmt::Display for Fqdn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

impl FromStr for Fqdn {
    type Err = FqdnError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Arbitrary upper limit of 100 characters so we know it will cleanly tls-serialize.
        if s.len() > 100 {
            return Err(FqdnError::NotADomainName);
        }
        match Host::parse(s)? {
            Host::Domain(_) => Ok(Self {
                domain: Host::<String>::parse(s)?,
            }),
            // Fqdns can't be IP addresses.
            Host::Ipv4(_) | Host::Ipv6(_) => Err(FqdnError::NotADomainName),
        }
    }
}

#[derive(Debug, Clone, PartialEq, TlsSerialize, TlsSize, TlsDeserializeBytes)]
pub struct QualifiedGroupId {
    group_uuid: [u8; 16],
    owning_domain: Fqdn,
}

impl QualifiedGroupId {
    pub fn new(uuid: Uuid, owning_domain: Fqdn) -> Self {
        let group_id = uuid.into_bytes();
        Self {
            group_uuid: group_id,
            owning_domain,
        }
    }

    pub fn group_uuid(&self) -> Uuid {
        Uuid::from_bytes(self.group_uuid)
    }

    pub fn owning_domain(&self) -> &Fqdn {
        &self.owning_domain
    }
}

impl TryFrom<GroupId> for QualifiedGroupId {
    type Error = tls_codec::Error;

    fn try_from(value: GroupId) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

impl TryFrom<&GroupId> for QualifiedGroupId {
    type Error = tls_codec::Error;

    fn try_from(value: &GroupId) -> Result<Self, Self::Error> {
        Self::tls_deserialize_exact_bytes(value.as_slice())
    }
}

impl From<QualifiedGroupId> for GroupId {
    fn from(value: QualifiedGroupId) -> Self {
        // We can unwrap here, because we know that neither the uuid nor the
        // domain will be too long to TLS-serialize.
        let group_id_bytes = value.tls_serialize_detached().unwrap();
        GroupId::from_slice(&group_id_bytes)
    }
}

impl fmt::Display for QualifiedGroupId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let uuid = Uuid::from_bytes(self.group_uuid);
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

impl FromStr for QualifiedGroupId {
    type Err = QualifiedGroupIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split_string = s.split('@');
        let group_id = split_string.next().ok_or_else(|| {
            debug!("The given string is empty");
            QualifiedGroupIdError::InvalidQualifiedGroupId
        })?;

        let group_id_uuid: Uuid = group_id.parse().map_err(|_| {
            debug!("The given group id is not a valid UUID");
            QualifiedGroupIdError::InvalidQualifiedGroupId
        })?;
        let group_id = group_id_uuid.into_bytes();
        // GroupIds MUST be qualified
        let domain = split_string.next().ok_or_else(|| {
            debug!("The given group id is not qualified");
            QualifiedGroupIdError::InvalidQualifiedGroupId
        })?;
        let owning_domain = domain.parse()?;
        if split_string.next().is_some() {
            debug!("The domain name may not contain a '@'");
            return Err(QualifiedGroupIdError::InvalidQualifiedGroupId);
        }

        Ok(Self {
            group_uuid: group_id,
            owning_domain,
        })
    }
}

#[derive(
    Clone,
    Debug,
    TlsSerialize,
    TlsSize,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    sqlx::Type,
)]
#[sqlx(transparent)]
pub struct UserName(TlsString);

impl fmt::Display for UserName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Error)]
pub enum UserNameError {
    #[error("The given string does not represent a valid user name")]
    InvalidUserName,
}

impl TryFrom<String> for UserName {
    type Error = UserNameError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.contains(['@', '.']) {
            Err(UserNameError::InvalidUserName)
        } else {
            Ok(Self(TlsString(value)))
        }
    }
}

impl From<UserName> for String {
    fn from(value: UserName) -> Self {
        value.0.0
    }
}

#[derive(
    Clone,
    Debug,
    TlsSerialize,
    TlsSize,
    TlsDeserializeBytes,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    sqlx::Type, // only for postgres
)]
#[sqlx(type_name = "qualified_user_name")]
pub struct QualifiedUserName {
    user_name: UserName,
    domain: Fqdn,
}

impl QualifiedUserName {
    pub fn new(user_name: UserName, domain: Fqdn) -> Self {
        Self { user_name, domain }
    }

    pub fn into_parts(self) -> (UserName, Fqdn) {
        (self.user_name, self.domain)
    }
}

impl Type<Sqlite> for QualifiedUserName {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <String as Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for QualifiedUserName {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        let value = self.to_string();
        Encode::<Sqlite>::encode(value, buf)
    }
}

impl<'r> Decode<'r, Sqlite> for QualifiedUserName {
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let s: &str = Decode::<Sqlite>::decode(value)?;
        Ok(s.parse()?)
    }
}

#[derive(Debug, Clone, Error)]
pub enum QualifiedUserNameError {
    #[error("Invalid string representation of qualified user name")]
    InvalidString,
    #[error(transparent)]
    InvalidUserName(#[from] UserNameError),
    #[error(transparent)]
    InvalidFqdn(#[from] FqdnError),
}

impl FromStr for QualifiedUserName {
    type Err = QualifiedUserNameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split_name = s.split('@');
        let user_name_str = split_name
            .next()
            .ok_or(QualifiedUserNameError::InvalidString)?;
        let user_name = UserName::try_from(user_name_str.to_string())?;
        // UserNames MUST be qualified
        let domain = split_name
            .next()
            .ok_or(QualifiedUserNameError::InvalidString)?;
        if split_name.next().is_some() {
            return Err(QualifiedUserNameError::InvalidString);
        }
        let domain = domain.parse()?;
        Ok(QualifiedUserName { user_name, domain })
    }
}

impl QualifiedUserName {
    pub fn user_name(&self) -> &UserName {
        &self.user_name
    }

    pub fn domain(&self) -> &Fqdn {
        &self.domain
    }
}

impl fmt::Display for QualifiedUserName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.user_name, self.domain)
    }
}

#[derive(
    Clone,
    Debug,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    Hash,
    TlsSize,
    TlsSerialize,
    TlsDeserializeBytes,
    sqlx::Type, // Only for Postgres
)]
#[sqlx(type_name = "as_client_id")]
pub struct AsClientId {
    user_name: QualifiedUserName,
    client_id: TlsUuid,
}

impl fmt::Display for AsClientId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let client_id_str = self.client_id.to_string();
        write!(f, "{}.{}", client_id_str, self.user_name)
    }
}

impl AsClientId {
    pub fn new(user_name: QualifiedUserName, client_id: Uuid) -> Self {
        Self {
            user_name,
            client_id: TlsUuid(client_id),
        }
    }

    pub fn random(user_name: QualifiedUserName) -> Result<Self, RandomnessError> {
        Ok(Self::new(user_name, Uuid::new_v4()))
    }

    pub fn user_name(&self) -> &QualifiedUserName {
        &self.user_name
    }

    pub fn client_id(&self) -> Uuid {
        *self.client_id
    }

    pub fn into_parts(self) -> (QualifiedUserName, Uuid) {
        (self.user_name, *self.client_id)
    }
}

#[derive(Debug, Clone, Error)]
pub enum AsClientIdError {
    #[error("The given string does not represent a valid client id")]
    InvalidClientId,
    #[error("The UUID of this client id is invalid: {0}")]
    InvalidClientUuid(#[from] uuid::Error),
    #[error("The user name of the client id is invalid: {0}")]
    UserNameError(#[from] QualifiedUserNameError),
}

impl FromStr for AsClientId {
    type Err = AsClientIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some((client_id_str, user_name_str)) = s.split_once('.') else {
            return Err(AsClientIdError::InvalidClientId);
        };
        let client_id = TlsUuid(Uuid::parse_str(client_id_str)?);
        let user_name = user_name_str.parse()?;
        Ok(Self {
            user_name,
            client_id,
        })
    }
}

impl Type<Sqlite> for AsClientId {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <String as Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for AsClientId {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        let value = self.to_string();
        Encode::<Sqlite>::encode(value, buf)
    }
}

impl<'r> Decode<'r, Sqlite> for AsClientId {
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let s: &str = Decode::<Sqlite>::decode(value)?;
        Ok(s.parse()?)
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
pub struct QsReference {
    pub client_homeserver_domain: Fqdn,
    pub sealed_reference: SealedClientReference,
}

#[derive(
    Debug, Serialize, Deserialize, Clone, TlsSerialize, TlsDeserializeBytes, TlsSize, PartialEq, Eq,
)]
pub struct SealedClientReference {
    pub(crate) ciphertext: HpkeCiphertext,
}

impl SealedClientReference {
    pub fn into_ciphertext(self) -> HpkeCiphertext {
        self.ciphertext
    }
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

impl HpkeEncryptable<ClientIdKeyType, SealedClientReference> for ClientConfig {}
impl HpkeDecryptable<ClientIdKeyType, SealedClientReference> for ClientConfig {}

/// This is the pseudonymous client id used on the QS.
#[derive(
    Serialize,
    Deserialize,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    TlsSize,
    TlsSerialize,
    TlsDeserializeBytes,
    sqlx::Type, // Only for Postgres
)]
#[sqlx(transparent)]
pub struct QsClientId(TlsUuid);

impl QsClientId {
    pub fn random(rng: &mut (impl CryptoRng + RngCore)) -> Self {
        let random_bytes = rng.r#gen::<[u8; 16]>();
        Uuid::from_bytes(random_bytes).into()
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl From<Uuid> for QsClientId {
    fn from(value: Uuid) -> Self {
        Self(TlsUuid(value))
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Hash,
    TlsSize,
    TlsDeserializeBytes,
    TlsSerialize,
    sqlx::Type,
)]
#[sqlx(transparent)]
pub struct QsUserId(TlsUuid);

impl From<Uuid> for QsUserId {
    fn from(value: Uuid) -> Self {
        Self(TlsUuid(value))
    }
}

impl QsUserId {
    pub fn random() -> Self {
        Uuid::new_v4().into()
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_fqdn() {
        let fqdn_str = "example.com";
        let fqdn = Fqdn::from_str(fqdn_str).unwrap();
        assert_eq!(fqdn.domain, Host::Domain(fqdn_str.to_string()));

        let fqdn_subdomain_str = "sub.example.com";
        let fqdn = Fqdn::from_str(fqdn_subdomain_str).unwrap();
        assert_eq!(fqdn.domain, Host::Domain(fqdn_subdomain_str.to_string()));
    }

    #[test]
    fn invalid_fqdn() {
        let fqdn_str = "invalid#domain#character";
        let result = Fqdn::from_str(fqdn_str);
        assert!(result.is_err());
        assert!(matches!(result, Err(FqdnError::UrlError(_))));
    }

    #[test]
    fn ip_address_fqdn() {
        let fqdn_str = "192.168.0.1";
        let result = Fqdn::from_str(fqdn_str);
        assert!(matches!(result, Err(FqdnError::NotADomainName)));
    }

    #[test]
    fn valid_user_name() {
        let user_name = UserName::try_from("alice".to_string());
        assert_eq!(user_name.unwrap().0.0, "alice");
    }

    #[test]
    fn invalid_user_name() {
        let user_name = UserName::try_from("alice@host".to_string());
        assert!(matches!(user_name, Err(UserNameError::InvalidUserName)));

        let user_name = UserName::try_from("alice.bob".to_string());
        assert!(matches!(user_name, Err(UserNameError::InvalidUserName)));
    }
}
