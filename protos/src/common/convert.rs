use chrono::DateTime;

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

impl From<GroupId> for openmls::group::GroupId {
    fn from(value: GroupId) -> Self {
        openmls::group::GroupId::from_slice(&value.value)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum QualifiedGroupIdError {
    #[error("missing group uuid")]
    MissingGroupUuid,
    #[error("missing domain")]
    MissingDomain,
    #[error(transparent)]
    InvalidDomain(#[from] phnxtypes::identifiers::FqdnError),
}

impl TryFrom<&Fqdn> for phnxtypes::identifiers::Fqdn {
    type Error = phnxtypes::identifiers::FqdnError;

    fn try_from(value: &Fqdn) -> Result<Self, Self::Error> {
        let domain = value.value.parse()?;
        Ok(domain)
    }
}

impl From<&phnxtypes::identifiers::Fqdn> for Fqdn {
    fn from(value: &phnxtypes::identifiers::Fqdn) -> Self {
        Self {
            value: value.to_string(),
        }
    }
}

impl TryFrom<&QualifiedGroupId> for phnxtypes::identifiers::QualifiedGroupId {
    type Error = QualifiedGroupIdError;

    fn try_from(value: &QualifiedGroupId) -> Result<Self, Self::Error> {
        Ok(Self::new(
            value
                .group_uuid
                .ok_or(QualifiedGroupIdError::MissingGroupUuid)?
                .into(),
            value
                .domain
                .as_ref()
                .ok_or(QualifiedGroupIdError::MissingDomain)?
                .try_into()?,
        ))
    }
}

impl From<&phnxtypes::identifiers::QualifiedGroupId> for QualifiedGroupId {
    fn from(value: &phnxtypes::identifiers::QualifiedGroupId) -> Self {
        QualifiedGroupId {
            group_uuid: Some(value.group_uuid().into()),
            domain: Some(value.owning_domain().into()),
        }
    }
}

impl From<openmls::group::GroupId> for GroupId {
    fn from(group_id: openmls::group::GroupId) -> Self {
        Self {
            value: group_id.to_vec(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CiphertextError {
    #[error("Invalid ciphertext nonce length {0}")]
    InvalidNonceLength(usize),
}

impl TryFrom<Ciphertext> for phnxtypes::crypto::ear::Ciphertext {
    type Error = CiphertextError;

    fn try_from(ciphertext: Ciphertext) -> Result<Self, Self::Error> {
        let nonce_len = ciphertext.nonce.len();
        Ok(Self::new(
            ciphertext.ciphertext,
            ciphertext
                .nonce
                .try_into()
                .map_err(|_| CiphertextError::InvalidNonceLength(nonce_len))?,
        ))
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
