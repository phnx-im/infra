use tls_codec::{DeserializeBytes, Serialize};

use crate::common::v1::Ciphertext;

use super::v1::{
    AssistedMessage, EncryptedIdentityLinkKey, GroupEpoch, GroupStateEarKey, HpkeCiphertext,
    LeafNodeIndex, MlsMessage, QsReference, RatchetTree, SealedClientReference, SignaturePublicKey,
};

#[derive(Debug, thiserror::Error)]
pub enum QsReferenceError {
    #[error("Missing client homeserver domain")]
    MissingClientHomeserverDomain,
    #[error("Missing sealed reference")]
    MissingSealedReference,
    #[error(transparent)]
    InvalidClientHomeserverDomain(#[from] phnxtypes::identifiers::FqdnError),
    #[error("Invalid sealed reference: {0}")]
    InvalidHpkeCiphertext(tls_codec::Error),
}

impl TryFrom<QsReference> for phnxtypes::identifiers::QsReference {
    type Error = QsReferenceError;

    fn try_from(value: QsReference) -> Result<Self, Self::Error> {
        let client_homeserver_domain = value
            .client_homeserver_domain
            .as_ref()
            .ok_or(QsReferenceError::MissingClientHomeserverDomain)?
            .try_into()?;
        let sealed_reference_bytes = value
            .sealed_reference
            .ok_or(QsReferenceError::MissingSealedReference)?
            .ciphertext
            .ok_or(QsReferenceError::MissingSealedReference)?
            .tls;
        let sealed_reference =
            openmls::prelude::HpkeCiphertext::tls_deserialize_exact_bytes(&sealed_reference_bytes)
                .map_err(QsReferenceError::InvalidHpkeCiphertext)?
                .into();

        Ok(Self {
            client_homeserver_domain,
            sealed_reference,
        })
    }
}

impl TryFrom<&phnxtypes::identifiers::QsReference> for QsReference {
    type Error = tls_codec::Error;

    fn try_from(value: &phnxtypes::identifiers::QsReference) -> Result<Self, Self::Error> {
        Ok(Self {
            client_homeserver_domain: Some((&value.client_homeserver_domain).into()),
            sealed_reference: Some((&value.sealed_reference).try_into()?),
        })
    }
}

impl TryFrom<&phnxtypes::identifiers::SealedClientReference> for SealedClientReference {
    type Error = tls_codec::Error;

    fn try_from(
        value: &phnxtypes::identifiers::SealedClientReference,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            ciphertext: Some(HpkeCiphertext {
                tls: value.as_ref().tls_serialize_detached()?,
            }),
        })
    }
}

impl From<phnxtypes::crypto::ear::keys::EncryptedIdentityLinkKey> for EncryptedIdentityLinkKey {
    fn from(value: phnxtypes::crypto::ear::keys::EncryptedIdentityLinkKey) -> Self {
        let ciphertext: phnxtypes::crypto::ear::Ciphertext = value.into();
        Self {
            ciphertext: Some(ciphertext.into()),
        }
    }
}

impl From<phnxtypes::crypto::ear::Ciphertext> for Ciphertext {
    fn from(value: phnxtypes::crypto::ear::Ciphertext) -> Self {
        let (ciphertext, nonce) = value.into_parts();
        Self {
            ciphertext,
            nonce: nonce.to_vec(),
        }
    }
}

impl TryFrom<MlsMessage> for openmls::framing::MlsMessageIn {
    type Error = tls_codec::Error;

    fn try_from(value: MlsMessage) -> Result<Self, Self::Error> {
        Self::tls_deserialize_exact_bytes(&value.tls)
    }
}

impl TryFrom<&openmls::framing::MlsMessageOut> for MlsMessage {
    type Error = tls_codec::Error;

    fn try_from(value: &openmls::framing::MlsMessageOut) -> Result<Self, Self::Error> {
        Ok(Self {
            tls: value.tls_serialize_detached()?,
        })
    }
}

impl TryFrom<&openmls::treesync::RatchetTree> for RatchetTree {
    type Error = tls_codec::Error;

    fn try_from(tree: &openmls::treesync::RatchetTree) -> Result<Self, Self::Error> {
        let tls = tree.tls_serialize_detached()?;
        Ok(Self { tls })
    }
}

impl TryFrom<RatchetTree> for openmls::treesync::RatchetTreeIn {
    type Error = tls_codec::Error;

    fn try_from(value: RatchetTree) -> Result<Self, Self::Error> {
        Self::tls_deserialize_exact_bytes(&value.tls)
    }
}

impl TryFrom<openmls::treesync::RatchetTree> for RatchetTree {
    type Error = tls_codec::Error;

    fn try_from(tree: openmls::treesync::RatchetTree) -> Result<Self, Self::Error> {
        let tls = tree.tls_serialize_detached()?;
        Ok(Self { tls })
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Invalid group state EAR key length")]
pub struct InvalidGroupStateEarKeyLength(usize);

impl From<&phnxtypes::crypto::ear::keys::GroupStateEarKey> for GroupStateEarKey {
    fn from(key: &phnxtypes::crypto::ear::keys::GroupStateEarKey) -> Self {
        Self {
            key: key.as_ref().secret().to_vec(),
        }
    }
}

impl TryFrom<&GroupStateEarKey> for phnxtypes::crypto::ear::keys::GroupStateEarKey {
    type Error = InvalidGroupStateEarKeyLength;

    fn try_from(value: &GroupStateEarKey) -> Result<Self, Self::Error> {
        let bytes: [u8; 32] = value
            .key
            .as_slice()
            .try_into()
            .map_err(|_| InvalidGroupStateEarKeyLength(value.key.len()))?;
        let key = phnxtypes::crypto::ear::keys::GroupStateEarKeySecret::from(bytes);
        Ok(key.into())
    }
}

impl From<GroupEpoch> for openmls::group::GroupEpoch {
    fn from(epoch: GroupEpoch) -> Self {
        epoch.value.into()
    }
}

impl From<openmls::group::GroupEpoch> for GroupEpoch {
    fn from(epoch: openmls::group::GroupEpoch) -> Self {
        Self {
            value: epoch.as_u64(),
        }
    }
}

impl From<LeafNodeIndex> for openmls::prelude::LeafNodeIndex {
    fn from(leaf_node_index: LeafNodeIndex) -> Self {
        Self::new(leaf_node_index.index)
    }
}

impl From<openmls::prelude::LeafNodeIndex> for LeafNodeIndex {
    fn from(leaf_node_index: openmls::prelude::LeafNodeIndex) -> Self {
        Self {
            index: leaf_node_index.u32(),
        }
    }
}

impl TryFrom<&AssistedMessage> for mls_assist::messages::AssistedMessageIn {
    type Error = tls_codec::Error;

    fn try_from(value: &AssistedMessage) -> Result<Self, Self::Error> {
        Self::tls_deserialize_exact_bytes(&value.tls)
    }
}

impl TryFrom<mls_assist::messages::AssistedMessageOut> for AssistedMessage {
    type Error = tls_codec::Error;

    fn try_from(value: mls_assist::messages::AssistedMessageOut) -> Result<Self, Self::Error> {
        let tls = value.tls_serialize_detached()?;
        Ok(Self { tls })
    }
}

impl From<SignaturePublicKey> for openmls::prelude::SignaturePublicKey {
    fn from(value: SignaturePublicKey) -> Self {
        value.bytes.into()
    }
}

impl From<&openmls::prelude::SignaturePublicKey> for SignaturePublicKey {
    fn from(value: &openmls::prelude::SignaturePublicKey) -> Self {
        Self {
            bytes: value.as_slice().to_vec(),
        }
    }
}
