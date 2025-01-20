// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{fmt, hash::Hash, str::FromStr};

use mls_assist::{openmls::group::GroupId, openmls_traits::types::HpkeCiphertext};
use rand::{CryptoRng, Rng, RngCore};
#[cfg(feature = "sqlite")]
use rusqlite::{
    types::{FromSql, FromSqlError},
    ToSql,
};
use tls_codec_impls::{TlsString, TlsUuid};
use tracing::{debug, error};
use url::Host;
use uuid::Uuid;

use crate::crypto::{
    ear::keys::PushTokenEarKey,
    errors::RandomnessError,
    hpke::{ClientIdDecryptionKey, ClientIdEncryptionKey, HpkeDecryptable, HpkeEncryptable},
};

use super::*;

mod tls_codec_impls;

pub const QS_CLIENT_REFERENCE_EXTENSION_TYPE: u16 = 0xff00;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Fqdn {
    domain: Host<String>,
}

#[cfg(feature = "sqlx")]
impl sqlx::Type<sqlx::Postgres> for Fqdn {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        String::type_info()
    }
}

#[cfg(feature = "sqlx")]
impl sqlx::Encode<'_, sqlx::Postgres> for Fqdn {
    fn encode_by_ref(
        &self,
        buf: &mut sqlx::postgres::PgArgumentBuffer,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Sync + Send>> {
        self.to_string().encode_by_ref(buf)
    }
}

#[cfg(feature = "sqlx")]
impl<'r> sqlx::Decode<'r, sqlx::Postgres> for Fqdn {
    fn decode(
        value: sqlx::postgres::PgValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let s: &str = sqlx::Decode::decode(value)?;
        let fqdn = s.parse().inspect_err(|error| {
            error!(%error, "Error parsing Fqdn from DB");
        })?;
        Ok(fqdn)
    }
}

#[cfg(feature = "sqlite")]
impl ToSql for Fqdn {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let string = self.to_string();
        Ok(rusqlite::types::ToSqlOutput::from(string))
    }
}

#[cfg(feature = "sqlite")]
impl FromSql for Fqdn {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let s = value.as_str()?;
        let fqdn = s.parse().map_err(|error| {
            error!(%error, "Error parsing Fqdn from DB");
            FromSqlError::InvalidType
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
)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
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
)]
#[cfg_attr(
    feature = "sqlx",
    derive(sqlx::Type),
    sqlx(type_name = "qualified_user_name")
)]
pub struct QualifiedUserName {
    user_name: UserName,
    domain: Fqdn,
}

#[cfg(feature = "sqlite")]
impl ToSql for QualifiedUserName {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let string = self.to_string();
        Ok(rusqlite::types::ToSqlOutput::from(string))
    }
}

#[cfg(feature = "sqlite")]
impl FromSql for QualifiedUserName {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let s = value.as_str()?;
        let user_name: QualifiedUserName = s.parse().map_err(|error| {
            error!(%error, "Error parsing UserName");
            FromSqlError::InvalidType
        })?;
        Ok(user_name)
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

    pub fn domain(&self) -> Fqdn {
        self.domain.clone()
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
)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(type_name = "as_client_id"))]
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

    pub fn user_name(&self) -> QualifiedUserName {
        // TODO: avoid this clone
        self.user_name.clone()
    }

    pub fn client_id(&self) -> Uuid {
        *self.client_id
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

impl TryFrom<String> for AsClientId {
    type Error = AsClientIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let Some((client_id_str, user_name_str)) = value.split_once('.') else {
            return Err(AsClientIdError::InvalidClientId);
        };
        let client_id = TlsUuid(Uuid::parse_str(client_id_str)?);
        let user_name: QualifiedUserName = user_name_str.parse()?;
        Ok(Self {
            user_name,
            client_id,
        })
    }
}

#[cfg(feature = "sqlite")]
impl ToSql for AsClientId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let string = self.to_string();
        Ok(rusqlite::types::ToSqlOutput::from(string))
    }
}

#[cfg(feature = "sqlite")]
impl FromSql for AsClientId {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let string = value.as_str()?.to_owned();
        let as_client_id = AsClientId::try_from(string).map_err(|e| {
            error!("Error parsing AsClientId: {}", e);
            FromSqlError::InvalidType
        })?;
        Ok(as_client_id)
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
#[derive(
    Serialize,
    Deserialize,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Hash,
    TlsSize,
    TlsSerialize,
    TlsDeserializeBytes,
)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
pub struct QsClientId(TlsUuid);

#[cfg(feature = "sqlite")]
impl ToSql for QsClientId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.0.to_sql()
    }
}

#[cfg(feature = "sqlite")]
impl FromSql for QsClientId {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        Uuid::column_result(value).map(|id| id.into())
    }
}

impl QsClientId {
    pub fn random(rng: &mut (impl CryptoRng + RngCore)) -> Self {
        let random_bytes = rng.gen::<[u8; 16]>();
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
    Debug,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Hash,
    TlsSize,
    TlsDeserializeBytes,
    TlsSerialize,
)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
pub struct QsUserId(TlsUuid);

#[cfg(feature = "sqlite")]
impl ToSql for QsUserId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.0.to_sql()
    }
}

#[cfg(feature = "sqlite")]
impl FromSql for QsUserId {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        Uuid::column_result(value).map(|id| id.into())
    }
}

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
        assert_eq!(user_name.unwrap().0 .0, "alice");
    }

    #[test]
    fn invalid_user_name() {
        let user_name = UserName::try_from("alice@host".to_string());
        assert!(matches!(user_name, Err(UserNameError::InvalidUserName)));

        let user_name = UserName::try_from("alice.bob".to_string());
        assert!(matches!(user_name, Err(UserNameError::InvalidUserName)));
    }
}
