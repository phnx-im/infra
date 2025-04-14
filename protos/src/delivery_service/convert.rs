use std::fmt;

use phnxtypes::{crypto::ear, identifiers};
use tls_codec::{DeserializeBytes, Serialize};
use tonic::Status;

use crate::{
    ToProto, TryToProto,
    common::{convert::InvalidNonceLen, v1::Ciphertext},
    error::{MissingFieldError, MissingFieldExt},
};

use super::v1::{
    AssistedMessage, EncryptedIdentityLinkKey, GroupEpoch, GroupStateEarKey, HpkeCiphertext,
    LeafNodeIndex, MlsMessage, QsReference, RatchetTree, SealedClientReference, SignaturePublicKey,
};

impl TryToProto<HpkeCiphertext> for openmls::prelude::HpkeCiphertext {
    type Error = tls_codec::Error;

    fn try_to_proto(&self) -> Result<HpkeCiphertext, Self::Error> {
        Ok(HpkeCiphertext {
            tls: self.tls_serialize_detached()?,
        })
    }
}

impl HpkeCiphertext {
    pub fn try_to_typed(&self) -> Result<openmls::prelude::HpkeCiphertext, tls_codec::Error> {
        DeserializeBytes::tls_deserialize_exact_bytes(&self.tls)
    }
}

impl TryToProto<SealedClientReference> for identifiers::SealedClientReference {
    type Error = tls_codec::Error;

    fn try_to_proto(&self) -> Result<SealedClientReference, Self::Error> {
        Ok(SealedClientReference {
            ciphertext: Some(self.as_ref().try_to_proto()?),
        })
    }
}

impl SealedClientReference {
    pub fn try_to_typed(
        &self,
    ) -> Result<identifiers::SealedClientReference, SealedClientReferenceError> {
        Ok(self
            .ciphertext
            .as_ref()
            .ok_or_missing_field(CiphertextField)?
            .try_to_typed()?
            .into())
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

impl TryToProto<QsReference> for identifiers::QsReference {
    type Error = tls_codec::Error;

    fn try_to_proto(&self) -> Result<QsReference, Self::Error> {
        Ok(QsReference {
            client_homeserver_domain: Some(self.client_homeserver_domain.to_proto()),
            sealed_reference: Some(self.sealed_reference.try_to_proto()?),
        })
    }
}

impl QsReference {
    pub fn try_to_typed(&self) -> Result<identifiers::QsReference, QsReferenceError> {
        use QsReferenceField::*;
        Ok(identifiers::QsReference {
            client_homeserver_domain: self
                .client_homeserver_domain
                .as_ref()
                .ok_or_missing_field(ClientHomeserverDomain)?
                .try_to_typed()?,
            sealed_reference: self
                .sealed_reference
                .as_ref()
                .ok_or_missing_field(SealedReference)?
                .try_to_typed()?,
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

impl EncryptedIdentityLinkKey {
    pub fn try_into_typed(
        self,
    ) -> Result<ear::keys::EncryptedIdentityLinkKey, EncryptedIdentityLinkKeyError<CiphertextField>>
    {
        let ciphertext: ear::Ciphertext = self
            .ciphertext
            .ok_or_missing_field(CiphertextField)?
            .try_into_typed()?;
        Ok(ear::keys::EncryptedIdentityLinkKey::from(ciphertext))
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

impl TryToProto<MlsMessage> for openmls::framing::MlsMessageOut {
    type Error = tls_codec::Error;

    fn try_to_proto(&self) -> Result<MlsMessage, Self::Error> {
        Ok(MlsMessage {
            tls: self.tls_serialize_detached()?,
        })
    }
}

impl MlsMessage {
    pub fn try_to_typed(&self) -> Result<openmls::framing::MlsMessageIn, tls_codec::Error> {
        DeserializeBytes::tls_deserialize_exact_bytes(&self.tls)
    }
}

impl TryToProto<RatchetTree> for openmls::treesync::RatchetTree {
    type Error = tls_codec::Error;

    fn try_to_proto(&self) -> Result<RatchetTree, Self::Error> {
        Ok(RatchetTree {
            tls: self.tls_serialize_detached()?,
        })
    }
}

impl RatchetTree {
    pub fn try_to_typed(&self) -> Result<openmls::treesync::RatchetTreeIn, tls_codec::Error> {
        DeserializeBytes::tls_deserialize_exact_bytes(&self.tls)
    }
}

impl ToProto<GroupStateEarKey> for ear::keys::GroupStateEarKey {
    fn to_proto(&self) -> GroupStateEarKey {
        GroupStateEarKey {
            key: self.as_ref().secret().to_vec(),
        }
    }
}

impl GroupStateEarKey {
    pub fn try_to_typed(&self) -> Result<ear::keys::GroupStateEarKey, InvalidGroupStateEarKeyLen> {
        let bytes: [u8; 32] = self
            .key
            .as_slice()
            .try_into()
            .map_err(|_| InvalidGroupStateEarKeyLen(self.key.len()))?;
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

impl TryToProto<AssistedMessage> for mls_assist::messages::AssistedMessageOut {
    type Error = tls_codec::Error;

    fn try_to_proto(&self) -> Result<AssistedMessage, Self::Error> {
        Ok(AssistedMessage {
            tls: self.tls_serialize_detached()?,
        })
    }
}

impl AssistedMessage {
    pub fn try_to_typed(
        &self,
    ) -> Result<mls_assist::messages::AssistedMessageIn, tls_codec::Error> {
        DeserializeBytes::tls_deserialize_exact_bytes(&self.tls)
    }
}

impl TryFrom<mls_assist::messages::AssistedMessageOut> for AssistedMessage {
    type Error = tls_codec::Error;

    fn try_from(value: mls_assist::messages::AssistedMessageOut) -> Result<Self, Self::Error> {
        Ok(Self {
            tls: value.tls_serialize_detached()?,
        })
    }
}

impl ToProto<SignaturePublicKey> for openmls::prelude::SignaturePublicKey {
    fn to_proto(&self) -> SignaturePublicKey {
        SignaturePublicKey {
            bytes: self.as_slice().to_vec(),
        }
    }
}

impl SignaturePublicKey {
    pub fn into_typed(self) -> openmls::prelude::SignaturePublicKey {
        self.bytes.into()
    }
}
