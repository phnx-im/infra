// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use chrono::DateTime;
use phnxtypes::{
    crypto::{ear, signatures::signable},
    identifiers, time,
};
use tonic::Status;

use crate::{
    convert::{FromRef, RefInto, TryFromRef, TryRefInto},
    validation::{MissingFieldError, MissingFieldExt},
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
        let (hi, lo) = (u64::from_le(value.hi), u64::from_le(value.lo));
        uuid::Uuid::from_u64_pair(hi, lo)
    }
}

impl TryFromRef<'_, Fqdn> for identifiers::Fqdn {
    type Error = identifiers::FqdnError;

    fn try_from_ref(value: &Fqdn) -> Result<Self, Self::Error> {
        value.value.parse()
    }
}

impl FromRef<'_, identifiers::Fqdn> for Fqdn {
    fn from_ref(value: &identifiers::Fqdn) -> Self {
        Fqdn {
            value: value.to_string(),
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

impl TryFromRef<'_, QualifiedGroupId> for identifiers::QualifiedGroupId {
    type Error = QualifiedGroupIdError;

    fn try_from_ref(proto: &QualifiedGroupId) -> Result<Self, Self::Error> {
        use QualifiedGroupIdField::*;
        Ok(identifiers::QualifiedGroupId::new(
            proto.group_uuid.ok_or_missing_field(GroupUuid)?.into(),
            proto
                .domain
                .as_ref()
                .ok_or_missing_field(Domain)?
                .try_ref_into()?,
        ))
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

impl FromRef<'_, identifiers::QualifiedGroupId> for QualifiedGroupId {
    fn from_ref(value: &identifiers::QualifiedGroupId) -> QualifiedGroupId {
        QualifiedGroupId {
            group_uuid: Some(value.group_uuid().into()),
            domain: Some(value.owning_domain().ref_into()),
        }
    }
}

impl FromRef<'_, openmls::group::GroupId> for GroupId {
    fn from_ref(value: &openmls::group::GroupId) -> GroupId {
        GroupId {
            value: value.to_vec(),
        }
    }
}

impl TryFrom<Ciphertext> for ear::Ciphertext {
    type Error = InvalidNonceLen;

    fn try_from(proto: Ciphertext) -> Result<Self, Self::Error> {
        let nonce_len = proto.nonce.len();
        let nonce = proto
            .nonce
            .try_into()
            .map_err(|_| InvalidNonceLen(nonce_len))?;
        Ok(ear::Ciphertext::new(proto.ciphertext, nonce))
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

impl From<time::TimeStamp> for Timestamp {
    fn from(value: time::TimeStamp) -> Self {
        let seconds = value.as_ref().timestamp();
        let nanos = value
            .as_ref()
            .timestamp_subsec_nanos()
            .try_into()
            .unwrap_or_default();
        Self { seconds, nanos }
    }
}

impl From<Timestamp> for time::TimeStamp {
    fn from(value: Timestamp) -> Self {
        DateTime::from_timestamp(value.seconds, value.nanos.try_into().unwrap_or_default())
            .unwrap_or_default()
            .into()
    }
}

impl From<signable::Signature> for Signature {
    fn from(value: signable::Signature) -> Self {
        Self {
            value: value.into_bytes(),
        }
    }
}

impl From<Signature> for signable::Signature {
    fn from(value: Signature) -> Self {
        signable::Signature::from_bytes(value.value)
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
