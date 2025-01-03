// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls_traits::storage::{Entity, Key, StorageProvider, CURRENT_VERSION};
use phnxtypes::codec::PhnxCodec;
use rusqlite::{
    types::{FromSql, ToSqlOutput},
    Connection, ToSql,
};
use serde::{Deserialize, Serialize};

use super::{
    encryption_key_pairs::{
        StorableEncryptionKeyPair, StorableEncryptionKeyPairRef, StorableEncryptionPublicKeyRef,
    },
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

pub(crate) struct SqliteStorageProvider<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteStorageProvider<'a> {
    pub(crate) fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
}

#[derive(Debug, Serialize)]
pub(super) struct KeyRefWrapper<'a, T: Key<CURRENT_VERSION>>(pub &'a T);

impl<T: Key<CURRENT_VERSION>> ToSql for KeyRefWrapper<'_, T> {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let key_bytes = PhnxCodec::to_vec(&self.0)?;
        Ok(ToSqlOutput::Owned(rusqlite::types::Value::Blob(key_bytes)))
    }
}

pub(super) struct EntityWrapper<T: Entity<CURRENT_VERSION>>(pub T);

impl<T: Entity<CURRENT_VERSION>> FromSql for EntityWrapper<T> {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let entity = PhnxCodec::from_slice(value.as_blob()?)?;
        Ok(Self(entity))
    }
}

pub(super) struct EntityRefWrapper<'a, T: Entity<CURRENT_VERSION>>(pub &'a T);

impl<T: Entity<CURRENT_VERSION>> ToSql for EntityRefWrapper<'_, T> {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let entity_bytes = PhnxCodec::to_vec(&self.0)?;
        Ok(ToSqlOutput::Owned(rusqlite::types::Value::Blob(
            entity_bytes,
        )))
    }
}

pub(super) struct EntitySliceWrapper<'a, T: Entity<CURRENT_VERSION>>(pub &'a [T]);

impl<T: Entity<CURRENT_VERSION>> ToSql for EntitySliceWrapper<'_, T> {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let entity_bytes = PhnxCodec::to_vec(&self.0)?;
        Ok(ToSqlOutput::Owned(rusqlite::types::Value::Blob(
            entity_bytes,
        )))
    }
}

pub(super) struct EntityVecWrapper<T: Entity<CURRENT_VERSION>>(pub Vec<T>);

impl<T: Entity<CURRENT_VERSION>> FromSql for EntityVecWrapper<T> {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let entities = PhnxCodec::from_slice(value.as_blob()?)?;
        Ok(Self(entities))
    }
}

pub(super) struct StorableGroupIdRef<'a, GroupId: Key<CURRENT_VERSION>>(pub &'a GroupId);

impl StorageProvider<{ CURRENT_VERSION }> for SqliteStorageProvider<'_> {
    type Error = rusqlite::Error;

    fn write_mls_join_config<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        MlsGroupJoinConfig: openmls_traits::storage::traits::MlsGroupJoinConfig<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        config: &MlsGroupJoinConfig,
    ) -> Result<(), Self::Error> {
        StorableGroupDataRef(config).store(
            self.connection,
            group_id,
            GroupDataType::JoinGroupConfig,
        )
    }

    fn append_own_leaf_node<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        LeafNode: openmls_traits::storage::traits::LeafNode<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        leaf_node: &LeafNode,
    ) -> Result<(), Self::Error> {
        StorableLeafNodeRef(leaf_node).store(self.connection, group_id)
    }

    fn queue_proposal<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        ProposalRef: openmls_traits::storage::traits::ProposalRef<CURRENT_VERSION>,
        QueuedProposal: openmls_traits::storage::traits::QueuedProposal<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        proposal_ref: &ProposalRef,
        proposal: &QueuedProposal,
    ) -> Result<(), Self::Error> {
        StorableProposalRef(proposal_ref, proposal).store(self.connection, group_id)
    }

    fn write_tree<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        TreeSync: openmls_traits::storage::traits::TreeSync<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        tree: &TreeSync,
    ) -> Result<(), Self::Error> {
        StorableGroupDataRef(tree).store(self.connection, group_id, GroupDataType::Tree)
    }

    fn write_interim_transcript_hash<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        InterimTranscriptHash: openmls_traits::storage::traits::InterimTranscriptHash<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        interim_transcript_hash: &InterimTranscriptHash,
    ) -> Result<(), Self::Error> {
        StorableGroupDataRef(interim_transcript_hash).store(
            self.connection,
            group_id,
            GroupDataType::InterimTranscriptHash,
        )
    }

    fn write_context<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        GroupContext: openmls_traits::storage::traits::GroupContext<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        group_context: &GroupContext,
    ) -> Result<(), Self::Error> {
        StorableGroupDataRef(group_context).store(self.connection, group_id, GroupDataType::Context)
    }

    fn write_confirmation_tag<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        ConfirmationTag: openmls_traits::storage::traits::ConfirmationTag<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        confirmation_tag: &ConfirmationTag,
    ) -> Result<(), Self::Error> {
        StorableGroupDataRef(confirmation_tag).store(
            self.connection,
            group_id,
            GroupDataType::ConfirmationTag,
        )
    }

    fn write_group_state<
        GroupState: openmls_traits::storage::traits::GroupState<CURRENT_VERSION>,
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        group_state: &GroupState,
    ) -> Result<(), Self::Error> {
        StorableGroupDataRef(group_state).store(
            self.connection,
            group_id,
            GroupDataType::GroupState,
        )
    }

    fn write_message_secrets<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        MessageSecrets: openmls_traits::storage::traits::MessageSecrets<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        message_secrets: &MessageSecrets,
    ) -> Result<(), Self::Error> {
        StorableGroupDataRef(message_secrets).store(
            self.connection,
            group_id,
            GroupDataType::MessageSecrets,
        )?;
        Ok(())
    }

    fn write_resumption_psk_store<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        ResumptionPskStore: openmls_traits::storage::traits::ResumptionPskStore<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        resumption_psk_store: &ResumptionPskStore,
    ) -> Result<(), Self::Error> {
        StorableGroupDataRef(resumption_psk_store).store(
            self.connection,
            group_id,
            GroupDataType::ResumptionPskStore,
        )
    }

    fn write_own_leaf_index<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        LeafNodeIndex: openmls_traits::storage::traits::LeafNodeIndex<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        own_leaf_index: &LeafNodeIndex,
    ) -> Result<(), Self::Error> {
        StorableGroupDataRef(own_leaf_index).store(
            self.connection,
            group_id,
            GroupDataType::OwnLeafIndex,
        )?;
        Ok(())
    }

    fn write_group_epoch_secrets<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        GroupEpochSecrets: openmls_traits::storage::traits::GroupEpochSecrets<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        group_epoch_secrets: &GroupEpochSecrets,
    ) -> Result<(), Self::Error> {
        StorableGroupDataRef(group_epoch_secrets).store(
            self.connection,
            group_id,
            GroupDataType::GroupEpochSecrets,
        )?;
        Ok(())
    }

    fn write_signature_key_pair<
        SignaturePublicKey: openmls_traits::storage::traits::SignaturePublicKey<CURRENT_VERSION>,
        SignatureKeyPair: openmls_traits::storage::traits::SignatureKeyPair<CURRENT_VERSION>,
    >(
        &self,
        public_key: &SignaturePublicKey,
        signature_key_pair: &SignatureKeyPair,
    ) -> Result<(), Self::Error> {
        StorableSignatureKeyPairsRef(signature_key_pair).store(self.connection, public_key)
    }

    fn write_encryption_key_pair<
        EncryptionKey: openmls_traits::storage::traits::EncryptionKey<CURRENT_VERSION>,
        HpkeKeyPair: openmls_traits::storage::traits::HpkeKeyPair<CURRENT_VERSION>,
    >(
        &self,
        public_key: &EncryptionKey,
        key_pair: &HpkeKeyPair,
    ) -> Result<(), Self::Error> {
        StorableEncryptionKeyPairRef(key_pair).store(self.connection, public_key)
    }

    fn write_encryption_epoch_key_pairs<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        EpochKey: openmls_traits::storage::traits::EpochKey<CURRENT_VERSION>,
        HpkeKeyPair: openmls_traits::storage::traits::HpkeKeyPair<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        epoch: &EpochKey,
        leaf_index: u32,
        key_pairs: &[HpkeKeyPair],
    ) -> Result<(), Self::Error> {
        StorableEpochKeyPairsRef(key_pairs).store(self.connection, group_id, epoch, leaf_index)
    }

    fn write_key_package<
        HashReference: openmls_traits::storage::traits::HashReference<CURRENT_VERSION>,
        KeyPackage: openmls_traits::storage::traits::KeyPackage<CURRENT_VERSION>,
    >(
        &self,
        hash_ref: &HashReference,
        key_package: &KeyPackage,
    ) -> Result<(), Self::Error> {
        StorableKeyPackageRef(key_package).store(self.connection, hash_ref)
    }

    fn write_psk<
        PskId: openmls_traits::storage::traits::PskId<CURRENT_VERSION>,
        PskBundle: openmls_traits::storage::traits::PskBundle<CURRENT_VERSION>,
    >(
        &self,
        psk_id: &PskId,
        psk: &PskBundle,
    ) -> Result<(), Self::Error> {
        StorablePskBundleRef(psk).store(self.connection, psk_id)
    }

    fn mls_group_join_config<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        MlsGroupJoinConfig: openmls_traits::storage::traits::MlsGroupJoinConfig<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<MlsGroupJoinConfig>, Self::Error> {
        StorableGroupData::load(self.connection, group_id, GroupDataType::JoinGroupConfig)
    }

    fn own_leaf_nodes<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        LeafNode: openmls_traits::storage::traits::LeafNode<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Vec<LeafNode>, Self::Error> {
        StorableLeafNode::load(self.connection, group_id)
    }

    fn queued_proposal_refs<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        ProposalRef: openmls_traits::storage::traits::ProposalRef<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Vec<ProposalRef>, Self::Error> {
        StorableProposal::<u8, ProposalRef>::load_refs(self.connection, group_id)
    }

    fn queued_proposals<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        ProposalRef: openmls_traits::storage::traits::ProposalRef<CURRENT_VERSION>,
        QueuedProposal: openmls_traits::storage::traits::QueuedProposal<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Vec<(ProposalRef, QueuedProposal)>, Self::Error> {
        StorableProposal::load(self.connection, group_id)
    }

    fn tree<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        TreeSync: openmls_traits::storage::traits::TreeSync<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<TreeSync>, Self::Error> {
        StorableGroupData::load(self.connection, group_id, GroupDataType::Tree)
    }

    fn group_context<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        GroupContext: openmls_traits::storage::traits::GroupContext<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<GroupContext>, Self::Error> {
        StorableGroupData::load(self.connection, group_id, GroupDataType::Context)
    }

    fn interim_transcript_hash<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        InterimTranscriptHash: openmls_traits::storage::traits::InterimTranscriptHash<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<InterimTranscriptHash>, Self::Error> {
        StorableGroupData::load(
            self.connection,
            group_id,
            GroupDataType::InterimTranscriptHash,
        )
    }

    fn confirmation_tag<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        ConfirmationTag: openmls_traits::storage::traits::ConfirmationTag<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<ConfirmationTag>, Self::Error> {
        StorableGroupData::load(self.connection, group_id, GroupDataType::ConfirmationTag)
    }

    fn group_state<
        GroupState: openmls_traits::storage::traits::GroupState<CURRENT_VERSION>,
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<GroupState>, Self::Error> {
        StorableGroupData::load(self.connection, group_id, GroupDataType::GroupState)
    }

    fn message_secrets<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        MessageSecrets: openmls_traits::storage::traits::MessageSecrets<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<MessageSecrets>, Self::Error> {
        StorableGroupData::load(self.connection, group_id, GroupDataType::MessageSecrets)
    }

    fn resumption_psk_store<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        ResumptionPskStore: openmls_traits::storage::traits::ResumptionPskStore<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<ResumptionPskStore>, Self::Error> {
        StorableGroupData::load(self.connection, group_id, GroupDataType::ResumptionPskStore)
    }

    fn own_leaf_index<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        LeafNodeIndex: openmls_traits::storage::traits::LeafNodeIndex<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<LeafNodeIndex>, Self::Error> {
        StorableGroupData::load(self.connection, group_id, GroupDataType::OwnLeafIndex)
    }

    fn group_epoch_secrets<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        GroupEpochSecrets: openmls_traits::storage::traits::GroupEpochSecrets<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<GroupEpochSecrets>, Self::Error> {
        StorableGroupData::load(self.connection, group_id, GroupDataType::GroupEpochSecrets)
    }

    fn signature_key_pair<
        SignaturePublicKey: openmls_traits::storage::traits::SignaturePublicKey<CURRENT_VERSION>,
        SignatureKeyPair: openmls_traits::storage::traits::SignatureKeyPair<CURRENT_VERSION>,
    >(
        &self,
        public_key: &SignaturePublicKey,
    ) -> Result<Option<SignatureKeyPair>, Self::Error> {
        StorableSignatureKeyPairs::load(self.connection, public_key)
    }

    fn encryption_key_pair<
        HpkeKeyPair: openmls_traits::storage::traits::HpkeKeyPair<CURRENT_VERSION>,
        EncryptionKey: openmls_traits::storage::traits::EncryptionKey<CURRENT_VERSION>,
    >(
        &self,
        public_key: &EncryptionKey,
    ) -> Result<Option<HpkeKeyPair>, Self::Error> {
        StorableEncryptionKeyPair::load(self.connection, public_key)
    }

    fn encryption_epoch_key_pairs<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        EpochKey: openmls_traits::storage::traits::EpochKey<CURRENT_VERSION>,
        HpkeKeyPair: openmls_traits::storage::traits::HpkeKeyPair<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        epoch: &EpochKey,
        leaf_index: u32,
    ) -> Result<Vec<HpkeKeyPair>, Self::Error> {
        StorableEpochKeyPairs::load(self.connection, group_id, epoch, leaf_index)
    }

    fn key_package<
        KeyPackageRef: openmls_traits::storage::traits::HashReference<CURRENT_VERSION>,
        KeyPackage: openmls_traits::storage::traits::KeyPackage<CURRENT_VERSION>,
    >(
        &self,
        hash_ref: &KeyPackageRef,
    ) -> Result<Option<KeyPackage>, Self::Error> {
        StorableKeyPackage::load(self.connection, hash_ref)
    }

    fn psk<
        PskBundle: openmls_traits::storage::traits::PskBundle<CURRENT_VERSION>,
        PskId: openmls_traits::storage::traits::PskId<CURRENT_VERSION>,
    >(
        &self,
        psk_id: &PskId,
    ) -> Result<Option<PskBundle>, Self::Error> {
        StorablePskBundle::load(self.connection, psk_id)
    }

    fn remove_proposal<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        ProposalRef: openmls_traits::storage::traits::ProposalRef<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        proposal_ref: &ProposalRef,
    ) -> Result<(), Self::Error> {
        StorableGroupIdRef(group_id).delete_proposal(self.connection, proposal_ref)
    }

    fn delete_own_leaf_nodes<GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>>(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        StorableGroupIdRef(group_id).delete_leaf_nodes(self.connection)
    }

    fn delete_group_config<GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>>(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        StorableGroupIdRef(group_id)
            .delete_group_data(self.connection, GroupDataType::JoinGroupConfig)
    }

    fn delete_tree<GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>>(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        StorableGroupIdRef(group_id).delete_group_data(self.connection, GroupDataType::Tree)
    }

    fn delete_confirmation_tag<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        StorableGroupIdRef(group_id)
            .delete_group_data(self.connection, GroupDataType::ConfirmationTag)
    }

    fn delete_group_state<GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>>(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        StorableGroupIdRef(group_id).delete_group_data(self.connection, GroupDataType::GroupState)
    }

    fn delete_context<GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>>(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        StorableGroupIdRef(group_id).delete_group_data(self.connection, GroupDataType::Context)
    }

    fn delete_interim_transcript_hash<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        StorableGroupIdRef(group_id)
            .delete_group_data(self.connection, GroupDataType::InterimTranscriptHash)
    }

    fn delete_message_secrets<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        StorableGroupIdRef(group_id)
            .delete_group_data(self.connection, GroupDataType::MessageSecrets)
    }

    fn delete_all_resumption_psk_secrets<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        StorableGroupIdRef(group_id)
            .delete_group_data(self.connection, GroupDataType::ResumptionPskStore)
    }

    fn delete_own_leaf_index<GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>>(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        StorableGroupIdRef(group_id).delete_group_data(self.connection, GroupDataType::OwnLeafIndex)
    }

    fn delete_group_epoch_secrets<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        StorableGroupIdRef(group_id)
            .delete_group_data(self.connection, GroupDataType::GroupEpochSecrets)
    }

    fn clear_proposal_queue<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        ProposalRef: openmls_traits::storage::traits::ProposalRef<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        StorableGroupIdRef(group_id).delete_all_proposals(self.connection)?;
        Ok(())
    }

    fn delete_signature_key_pair<
        SignaturePublicKey: openmls_traits::storage::traits::SignaturePublicKey<CURRENT_VERSION>,
    >(
        &self,
        public_key: &SignaturePublicKey,
    ) -> Result<(), Self::Error> {
        StorableSignaturePublicKeyRef(public_key).delete(self.connection)
    }

    fn delete_encryption_key_pair<
        EncryptionKey: openmls_traits::storage::traits::EncryptionKey<CURRENT_VERSION>,
    >(
        &self,
        public_key: &EncryptionKey,
    ) -> Result<(), Self::Error> {
        StorableEncryptionPublicKeyRef(public_key).delete(self.connection)
    }

    fn delete_encryption_epoch_key_pairs<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        EpochKey: openmls_traits::storage::traits::EpochKey<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        epoch: &EpochKey,
        leaf_index: u32,
    ) -> Result<(), Self::Error> {
        StorableGroupIdRef(group_id).delete_epoch_key_pair(self.connection, epoch, leaf_index)
    }

    fn delete_key_package<
        KeyPackageRef: openmls_traits::storage::traits::HashReference<CURRENT_VERSION>,
    >(
        &self,
        hash_ref: &KeyPackageRef,
    ) -> Result<(), Self::Error> {
        StorableHashRef(hash_ref).delete_key_package(self.connection)
    }

    fn delete_psk<PskKey: openmls_traits::storage::traits::PskId<CURRENT_VERSION>>(
        &self,
        psk_id: &PskKey,
    ) -> Result<(), Self::Error> {
        StorablePskIdRef(psk_id).delete(self.connection)
    }
}

#[derive(Serialize, Deserialize)]
struct Aad(Vec<u8>);

impl Entity<CURRENT_VERSION> for Aad {}
