// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;

use phnxtypes::{crypto::ear, identifiers};
use tls_codec::{DeserializeBytes, Serialize};
use tonic::Status;

use crate::{
    common::{convert::InvalidNonceLen, v1::Ciphertext},
    convert::{FromRef, RefInto, TryFromRef, TryRefInto},
    validation::{MissingFieldError, MissingFieldExt},
};

use super::v1::{
    AssistedMessage, EncryptedIdentityLinkKey, GroupEpoch, GroupStateEarKey, HpkeCiphertext,
    LeafNodeIndex, MlsMessage, QsReference, RatchetTree, SealedClientReference, SignaturePublicKey,
};

impl TryFromRef<'_, openmls::prelude::HpkeCiphertext> for HpkeCiphertext {
    type Error = tls_codec::Error;

    fn try_from_ref(value: &openmls::prelude::HpkeCiphertext) -> Result<Self, Self::Error> {
        Ok(Self {
            tls: value.tls_serialize_detached()?,
        })
    }
}

impl TryFromRef<'_, HpkeCiphertext> for openmls::prelude::HpkeCiphertext {
    type Error = tls_codec::Error;

    fn try_from_ref(proto: &HpkeCiphertext) -> Result<Self, Self::Error> {
        DeserializeBytes::tls_deserialize_exact_bytes(&proto.tls)
    }
}

impl TryFromRef<'_, identifiers::SealedClientReference> for SealedClientReference {
    type Error = tls_codec::Error;

    fn try_from_ref(value: &identifiers::SealedClientReference) -> Result<Self, Self::Error> {
        Ok(Self {
            ciphertext: Some(value.as_ref().try_ref_into()?),
        })
    }
}

impl TryFromRef<'_, SealedClientReference> for identifiers::SealedClientReference {
    type Error = SealedClientReferenceError;

    fn try_from_ref(proto: &SealedClientReference) -> Result<Self, Self::Error> {
        let ciphertext = proto
            .ciphertext
            .as_ref()
            .ok_or_missing_field(CiphertextField)?;
        let ciphertext = openmls::prelude::HpkeCiphertext::try_from_ref(ciphertext)?;
        Ok(ciphertext.into())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SealedClientReferenceError {
    #[error(transparent)]
    Field(#[from] MissingFieldError<CiphertextField>),
    #[error(transparent)]
    InvalidCiphertext(#[from] tls_codec::Error),
}

impl From<SealedClientReferenceError> for Status {
    fn from(e: SealedClientReferenceError) -> Self {
        Status::invalid_argument(format!("invalid sealed client reference: {e}"))
    }
}

impl TryFromRef<'_, identifiers::QsReference> for QsReference {
    type Error = tls_codec::Error;

    fn try_from_ref(value: &identifiers::QsReference) -> Result<Self, Self::Error> {
        Ok(Self {
            client_homeserver_domain: Some(value.client_homeserver_domain.ref_into()),
            sealed_reference: Some(value.sealed_reference.try_ref_into()?),
        })
    }
}

impl TryFromRef<'_, QsReference> for identifiers::QsReference {
    type Error = QsReferenceError;

    fn try_from_ref(proto: &QsReference) -> Result<Self, Self::Error> {
        use QsReferenceField::*;
        Ok(Self {
            client_homeserver_domain: proto
                .client_homeserver_domain
                .as_ref()
                .ok_or_missing_field(ClientHomeserverDomain)?
                .try_ref_into()?,
            sealed_reference: proto
                .sealed_reference
                .as_ref()
                .ok_or_missing_field(SealedReference)?
                .try_ref_into()?,
        })
    }
}

#[derive(Debug, derive_more::Display)]
pub enum QsReferenceField {
    #[display(fmt = "client_homeserver_domain")]
    ClientHomeserverDomain,
    #[display(fmt = "sealed_reference")]
    SealedReference,
}

#[derive(Debug, thiserror::Error)]
pub enum QsReferenceError {
    #[error(transparent)]
    Field(#[from] MissingFieldError<QsReferenceField>),
    #[error(transparent)]
    Fqdn(#[from] identifiers::FqdnError),
    #[error(transparent)]
    SealedClientReference(#[from] SealedClientReferenceError),
}

impl From<QsReferenceError> for Status {
    fn from(e: QsReferenceError) -> Self {
        Status::invalid_argument(format!("invalid QS reference: {e}"))
    }
}

impl From<ear::keys::EncryptedIdentityLinkKey> for EncryptedIdentityLinkKey {
    fn from(value: ear::keys::EncryptedIdentityLinkKey) -> Self {
        let ciphertext: ear::Ciphertext = value.into();
        Self {
            ciphertext: Some(ciphertext.into()),
        }
    }
}

impl From<ear::Ciphertext> for Ciphertext {
    fn from(value: ear::Ciphertext) -> Self {
        let (ciphertext, nonce) = value.into_parts();
        Self {
            ciphertext,
            nonce: nonce.to_vec(),
        }
    }
}

impl TryFrom<EncryptedIdentityLinkKey> for ear::keys::EncryptedIdentityLinkKey {
    type Error = EncryptedIdentityLinkKeyError<CiphertextField>;

    fn try_from(proto: EncryptedIdentityLinkKey) -> Result<Self, Self::Error> {
        let ciphertext: ear::Ciphertext = proto
            .ciphertext
            .ok_or_missing_field(CiphertextField)?
            .try_into()?;
        Ok(ciphertext.into())
    }
}

#[derive(Debug, derive_more::Display)]
#[display(fmt = "ciphertext")]
pub struct CiphertextField;

#[derive(Debug, thiserror::Error)]
pub enum EncryptedIdentityLinkKeyError<E: fmt::Display> {
    #[error(transparent)]
    Field(#[from] MissingFieldError<E>),
    #[error(transparent)]
    Ciphertext(#[from] InvalidNonceLen),
}

impl<E: fmt::Display> From<EncryptedIdentityLinkKeyError<E>> for Status {
    fn from(e: EncryptedIdentityLinkKeyError<E>) -> Self {
        Status::invalid_argument(format!("invalid encrypted identity link key: {e}"))
    }
}

impl TryFromRef<'_, openmls::framing::MlsMessageOut> for MlsMessage {
    type Error = tls_codec::Error;

    fn try_from_ref(value: &openmls::framing::MlsMessageOut) -> Result<Self, Self::Error> {
        Ok(Self {
            tls: value.tls_serialize_detached()?,
        })
    }
}

impl TryFromRef<'_, MlsMessage> for openmls::framing::MlsMessageIn {
    type Error = tls_codec::Error;

    fn try_from_ref(proto: &MlsMessage) -> Result<Self, Self::Error> {
        DeserializeBytes::tls_deserialize_exact_bytes(&proto.tls)
    }
}

impl TryFromRef<'_, openmls::treesync::RatchetTree> for RatchetTree {
    type Error = tls_codec::Error;

    fn try_from_ref(value: &openmls::treesync::RatchetTree) -> Result<Self, Self::Error> {
        Ok(Self {
            tls: value.tls_serialize_detached()?,
        })
    }
}

impl TryFromRef<'_, RatchetTree> for openmls::treesync::RatchetTreeIn {
    type Error = tls_codec::Error;

    fn try_from_ref(proto: &RatchetTree) -> Result<Self, Self::Error> {
        DeserializeBytes::tls_deserialize_exact_bytes(&proto.tls)
    }
}

impl FromRef<'_, ear::keys::GroupStateEarKey> for GroupStateEarKey {
    fn from_ref(value: &ear::keys::GroupStateEarKey) -> Self {
        Self {
            key: value.as_ref().secret().to_vec(),
        }
    }
}

impl TryFromRef<'_, GroupStateEarKey> for ear::keys::GroupStateEarKey {
    type Error = InvalidGroupStateEarKeyLen;

    fn try_from_ref(proto: &GroupStateEarKey) -> Result<Self, Self::Error> {
        let bytes: [u8; 32] = proto
            .key
            .as_slice()
            .try_into()
            .map_err(|_| InvalidGroupStateEarKeyLen(proto.key.len()))?;
        let key = ear::keys::GroupStateEarKeySecret::from(bytes);
        Ok(key.into())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Invalid group state EAR key length: {0}")]
pub struct InvalidGroupStateEarKeyLen(usize);

impl From<InvalidGroupStateEarKeyLen> for Status {
    fn from(e: InvalidGroupStateEarKeyLen) -> Self {
        Status::invalid_argument(e.to_string())
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

impl TryFromRef<'_, mls_assist::messages::AssistedMessageOut> for AssistedMessage {
    type Error = tls_codec::Error;

    fn try_from_ref(value: &mls_assist::messages::AssistedMessageOut) -> Result<Self, Self::Error> {
        Ok(Self {
            tls: value.tls_serialize_detached()?,
        })
    }
}

impl TryFromRef<'_, AssistedMessage> for mls_assist::messages::AssistedMessageIn {
    type Error = tls_codec::Error;

    fn try_from_ref(proto: &AssistedMessage) -> Result<Self, Self::Error> {
        DeserializeBytes::tls_deserialize_exact_bytes(&proto.tls)
    }
}

impl FromRef<'_, openmls::prelude::SignaturePublicKey> for SignaturePublicKey {
    fn from_ref(value: &openmls::prelude::SignaturePublicKey) -> Self {
        Self {
            bytes: value.as_slice().to_vec(),
        }
    }
}

impl From<SignaturePublicKey> for openmls::prelude::SignaturePublicKey {
    fn from(proto: SignaturePublicKey) -> Self {
        proto.bytes.into()
    }
}
