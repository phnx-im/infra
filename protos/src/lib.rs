pub mod delivery_service;

pub mod common {
    pub mod v1 {
        tonic::include_proto!("common.v1");

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

        impl TryFrom<Fqdn> for phnxtypes::identifiers::Fqdn {
            type Error = phnxtypes::identifiers::FqdnError;

            fn try_from(value: Fqdn) -> Result<Self, Self::Error> {
                let domain = value.value.parse()?;
                Ok(domain)
            }
        }

        impl TryFrom<QualifiedGroupId> for phnxtypes::identifiers::QualifiedGroupId {
            type Error = QualifiedGroupIdError;

            fn try_from(value: QualifiedGroupId) -> Result<Self, Self::Error> {
                Ok(Self::new(
                    value
                        .group_uuid
                        .ok_or(QualifiedGroupIdError::MissingGroupUuid)?
                        .into(),
                    value
                        .domain
                        .ok_or(QualifiedGroupIdError::MissingDomain)?
                        .try_into()?,
                ))
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
            #[error("Invalid ciphertext nonce length")]
            InvalidNonceLength,
        }

        impl TryFrom<Ciphertext> for phnxtypes::crypto::ear::Ciphertext {
            type Error = CiphertextError;

            fn try_from(ciphertext: Ciphertext) -> Result<Self, Self::Error> {
                Ok(Self::new(
                    ciphertext.ciphertext,
                    ciphertext
                        .nonce
                        .try_into()
                        .map_err(|_| CiphertextError::InvalidNonceLength)?,
                ))
            }
        }
    }
}

pub mod auth_service {
    pub mod v1 {
        tonic::include_proto!("auth_service.v1");
    }
}

pub mod queue_service {
    pub mod v1 {
        tonic::include_proto!("queue_service.v1");
    }
}

#[cfg(test)]
mod test {
    use crate::common::v1::Uuid;

    #[test]
    fn uuid_conversion() {
        let uuid = uuid::Uuid::parse_str("a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8").unwrap();
        let proto_uuid = Uuid::from(uuid);
        assert_eq!(uuid, uuid::Uuid::from(proto_uuid));
        assert_eq!(proto_uuid, Uuid::from(uuid));
    }
}
