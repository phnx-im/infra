use chrono::DateTime;
use phnxtypes::{crypto::ear, identifiers};
use tonic::Status;

use crate::{
    IntoProto, ToProto,
    error::{MissingFieldError, MissingFieldExt},
};

use super::v1::{Ciphertext, Fqdn, GroupId, QualifiedGroupId, Signature, Timestamp, Uuid};

impl From<uuid::Uuid> for Uuid {
    fn from(value: uuid::Uuid) -> Self {
        let (hi, lo) = value.as_u64_pair();
        Self {
            hi: hi.to_le(),
            lo: lo.to_le(),
        }
    }
}

impl From<Uuid> for uuid::Uuid {
    fn from(value: Uuid) -> Self {
        let (lo, hi) = (u64::from_le(value.lo), u64::from_le(value.hi));
        uuid::Uuid::from_u64_pair(hi, lo)
    }
}

impl GroupId {
    pub fn to_openmls(&self) -> openmls::group::GroupId {
        openmls::group::GroupId::from_slice(&self.value)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum QualifiedGroupIdError {
    #[error(transparent)]
    Field(#[from] MissingFieldError<QualifiedGroupIdField>),
    #[error(transparent)]
    Fqdn(#[from] identifiers::FqdnError),
}

impl From<QualifiedGroupIdError> for Status {
    fn from(e: QualifiedGroupIdError) -> Self {
        Status::invalid_argument(format!("invalid qualified group id: {e}"))
    }
}

impl Fqdn {
    pub fn try_to_typed(&self) -> Result<identifiers::Fqdn, identifiers::FqdnError> {
        self.value.parse()
    }
}

impl ToProto<Fqdn> for identifiers::Fqdn {
    fn to_proto(&self) -> Fqdn {
        Fqdn {
            value: self.to_string(),
        }
    }
}

#[derive(Debug, derive_more::Display)]
pub enum QualifiedGroupIdField {
    #[display(fmt = "group_uuid")]
    GroupUuid,
    #[display(fmt = "domain")]
    Domain,
}

impl QualifiedGroupId {
    pub fn try_to_typed(&self) -> Result<identifiers::QualifiedGroupId, QualifiedGroupIdError> {
        use QualifiedGroupIdField::*;
        Ok(identifiers::QualifiedGroupId::new(
            self.group_uuid.ok_or_missing_field(GroupUuid)?.into(),
            self.domain
                .as_ref()
                .ok_or_missing_field(Domain)?
                .try_to_typed()?,
        ))
    }
}

impl ToProto<QualifiedGroupId> for identifiers::QualifiedGroupId {
    fn to_proto(&self) -> QualifiedGroupId {
        QualifiedGroupId {
            group_uuid: Some(self.group_uuid().into_proto()),
            domain: Some(self.owning_domain().to_proto()),
        }
    }
}

impl ToProto<GroupId> for openmls::group::GroupId {
    fn to_proto(&self) -> GroupId {
        GroupId {
            value: self.to_vec(),
        }
    }
}

impl Ciphertext {
    pub fn try_into_typed(self) -> Result<ear::Ciphertext, InvalidNonceLen> {
        let nonce_len = self.nonce.len();
        let nonce = self
            .nonce
            .try_into()
            .map_err(|_| InvalidNonceLen(nonce_len))?;
        Ok(ear::Ciphertext::new(self.ciphertext, nonce))
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid ciphertext nonce length {0}")]
pub struct InvalidNonceLen(usize);

impl From<InvalidNonceLen> for Status {
    fn from(e: InvalidNonceLen) -> Self {
        Status::invalid_argument(format!("invalid ciphertext nonce length: {}", e.0))
    }
}

impl From<phnxtypes::time::TimeStamp> for Timestamp {
    fn from(value: phnxtypes::time::TimeStamp) -> Self {
        let seconds = value.as_ref().timestamp();
        let nanos = value
            .as_ref()
            .timestamp_subsec_nanos()
            .try_into()
            .unwrap_or_default();
        Self { seconds, nanos }
    }
}

impl From<Timestamp> for phnxtypes::time::TimeStamp {
    fn from(value: Timestamp) -> Self {
        DateTime::from_timestamp(value.seconds, value.nanos.try_into().unwrap_or_default())
            .unwrap_or_default()
            .into()
    }
}

impl From<phnxtypes::crypto::signatures::signable::Signature> for Signature {
    fn from(value: phnxtypes::crypto::signatures::signable::Signature) -> Self {
        Self {
            value: value.into_bytes(),
        }
    }
}

impl From<Signature> for phnxtypes::crypto::signatures::signable::Signature {
    fn from(value: Signature) -> Self {
        phnxtypes::crypto::signatures::signable::Signature::from_bytes(value.value)
    }
}

#[cfg(test)]
mod test {
    use uuid::uuid;

    use super::*;

    #[test]
    fn uuid_conversion() {
        let uuid = uuid!("a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8");
        let proto_uuid = Uuid::from(uuid);
        assert_eq!(uuid, uuid::Uuid::from(proto_uuid));
        assert_eq!(proto_uuid, Uuid::from(uuid));
    }
}
