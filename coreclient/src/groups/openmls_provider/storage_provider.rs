// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{cell::RefCell, future::Future};

use openmls_sqlx_storage::Codec;
use openmls_traits::storage::{
    CURRENT_VERSION, Entity, Key, StorageProvider,
    traits::{
        self, ProposalRef as ProposalRefTrait, SignaturePublicKey as SignaturePublicKeyTrait,
    },
};
use sqlx::{
    Database, Decode, Encode, Row, Sqlite, SqliteConnection, SqliteExecutor, Type, encode::IsNull,
    error::BoxDynError, query, sqlite::SqliteTypeInfo,
};
use tokio_stream::StreamExt;

use crate::groups::openmls_provider::encryption_key_pairs::StorableEncryptionKeyPairRef;

use super::{
    EntityRefWrapper, EntitySliceWrapper, EntityVecWrapper, EntityWrapper, KeyRefWrapper,
    StorableGroupIdRef,
    encryption_key_pairs::{StorableEncryptionKeyPair, StorableEncryptionPublicKeyRef},
    epoch_key_pairs::{StorableEpochKeyPairs, StorableEpochKeyPairsRef},
    group_data::{GroupDataType, StorableGroupData, StorableGroupDataRef},
    key_packages::{StorableHashRef, StorableKeyPackage, StorableKeyPackageRef},
    own_leaf_nodes::{StorableLeafNode, StorableLeafNodeRef},
    proposals::{StorableProposal, StorableProposalRef},
    psks::{StorablePskBundle, StorablePskBundleRef, StorablePskIdRef},
    signature_key_pairs::{
        StorableSignatureKeyPairs, StorableSignatureKeyPairsRef, StorableSignaturePublicKeyRef,
    },
};

#[derive(Debug, Default)]
pub(crate) struct PersistenceCodec;

impl Codec for PersistenceCodec {
    type Error = aircommon::codec::Error;

    fn to_vec<T: ?Sized + serde::Serialize>(value: &T) -> Result<Vec<u8>, Self::Error> {
        aircommon::codec::PersistenceCodec::to_vec(value)
    }

    fn from_slice<T: serde::de::DeserializeOwned>(slice: &[u8]) -> Result<T, Self::Error> {
        aircommon::codec::PersistenceCodec::from_slice(slice)
    }
}

pub(crate) type SqliteStorageProvider<'a> =
    openmls_sqlx_storage::SqliteStorageProvider<'a, PersistenceCodec>;
