// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls::key_packages::KeyPackageBundle;
use openmls_traits::storage::{Entity, Key, StorageProvider, CURRENT_VERSION};
use rusqlite::{
    types::{FromSql, ToSqlOutput},
    Connection, ToSql,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{
    encryption_key_pairs::StorableEncryptionKeyPair,
    epoch_key_pairs::StorableEpochKeyPairs,
    group_data::{GroupDataType, StorableGroupData, StorableGroupDataRef},
    key_packages::StorableKeyPackage,
    psks::StorablePskBundle,
    OwnLeafNode, StorableProposal, StorableSignatureKeyPairs,
};

pub(crate) struct SqliteStorageProvider<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteStorageProvider<'a> {
    pub(crate) fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
}

#[derive(Debug, Error, PartialEq)]
pub(crate) enum SqliteStorageProviderError {
    #[error("Sqlite error: {0}")]
    SqliteError(#[from] rusqlite::Error),
    #[error("Serde error")]
    SerdeError, //(#[from] serde_json::Error),
}

impl From<serde_json::Error> for SqliteStorageProviderError {
    fn from(_: serde_json::Error) -> Self {
        Self::SerdeError
    }
}

#[derive(Debug, Serialize)]
pub(super) struct KeyRefWrapper<'a, T: Key<CURRENT_VERSION>>(pub &'a T);

impl<T: Key<CURRENT_VERSION>> ToSql for KeyRefWrapper<'_, T> {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let key_bytes = serde_json::to_vec(&self.0).map_err(|e| {
            log::error!("Failed to serialize key: {}", e);
            rusqlite::Error::ToSqlConversionFailure(Box::new(e))
        })?;
        Ok(ToSqlOutput::Owned(rusqlite::types::Value::Blob(key_bytes)))
    }
}

pub(super) struct EntityWrapper<T: Entity<CURRENT_VERSION>>(pub T);

impl<T: Entity<CURRENT_VERSION>> FromSql for EntityWrapper<T> {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let entity = serde_json::from_slice(value.as_blob()?).map_err(|e| {
            log::error!("Failed to deserialize entity: {}", e);
            rusqlite::types::FromSqlError::Other(Box::new(e))
        })?;
        Ok(Self(entity))
    }
}

pub(super) struct EntityRefWrapper<'a, T: Entity<CURRENT_VERSION>>(pub &'a T);

impl<'a, T: Entity<CURRENT_VERSION>> ToSql for EntityRefWrapper<'a, T> {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let entity_bytes = serde_json::to_vec(&self.0).map_err(|e| {
            log::error!("Failed to serialize entity: {}", e);
            rusqlite::Error::ToSqlConversionFailure(Box::new(e))
        })?;
        Ok(ToSqlOutput::Owned(rusqlite::types::Value::Blob(
            entity_bytes,
        )))
    }
}

impl<'a> StorageProvider<{ CURRENT_VERSION }> for SqliteStorageProvider<'a> {
    type Error = SqliteStorageProviderError;

    fn write_mls_join_config<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        MlsGroupJoinConfig: openmls_traits::storage::traits::MlsGroupJoinConfig<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        config: &MlsGroupJoinConfig,
    ) -> Result<(), Self::Error> {
        StorableGroupDataRef::new(group_id, GroupDataType::JoinGroupConfig, config)
            .store(self.connection)?;
        Ok(())
    }

    fn write_aad<GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>>(
        &self,
        group_id: &GroupId,
        aad: &[u8],
    ) -> Result<(), Self::Error> {
        StorableGroupDataRef::new(group_id, GroupDataType::Aad, &Aad(aad.to_vec()))
            .store(self.connection)?;
        Ok(())
    }

    fn append_own_leaf_node<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        LeafNode: openmls_traits::storage::traits::LeafNode<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        leaf_node: &LeafNode,
    ) -> Result<(), Self::Error> {
        OwnLeafNode::new(leaf_node)?.store(self.connection, group_id)?;
        Ok(())
    }

    fn clear_own_leaf_nodes<GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>>(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        OwnLeafNode::delete(self.connection, group_id)?;
        Ok(())
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
        StorableProposal::new(proposal_ref, proposal)?.store(self.connection, group_id)?;
        Ok(())
    }

    fn write_tree<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        TreeSync: openmls_traits::storage::traits::TreeSync<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        tree: &TreeSync,
    ) -> Result<(), Self::Error> {
        StorableGroupDataRef::new(group_id, GroupDataType::Tree, tree).store(self.connection)?;
        Ok(())
    }

    fn write_interim_transcript_hash<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        InterimTranscriptHash: openmls_traits::storage::traits::InterimTranscriptHash<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        interim_transcript_hash: &InterimTranscriptHash,
    ) -> Result<(), Self::Error> {
        StorableGroupDataRef::new(
            group_id,
            GroupDataType::InterimTranscriptHash,
            interim_transcript_hash,
        )
        .store(self.connection)?;
        Ok(())
    }

    fn write_context<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        GroupContext: openmls_traits::storage::traits::GroupContext<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        group_context: &GroupContext,
    ) -> Result<(), Self::Error> {
        StorableGroupDataRef::new(group_id, GroupDataType::Context, group_context)
            .store(self.connection)?;
        Ok(())
    }

    fn write_confirmation_tag<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        ConfirmationTag: openmls_traits::storage::traits::ConfirmationTag<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        confirmation_tag: &ConfirmationTag,
    ) -> Result<(), Self::Error> {
        StorableGroupDataRef::new(group_id, GroupDataType::ConfirmationTag, confirmation_tag)
            .store(self.connection)?;
        Ok(())
    }

    fn write_group_state<
        GroupState: openmls_traits::storage::traits::GroupState<CURRENT_VERSION>,
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        group_state: &GroupState,
    ) -> Result<(), Self::Error> {
        StorableGroupDataRef::new(group_id, GroupDataType::GroupState, group_state)
            .store(self.connection)?;
        Ok(())
    }

    fn write_message_secrets<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        MessageSecrets: openmls_traits::storage::traits::MessageSecrets<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        message_secrets: &MessageSecrets,
    ) -> Result<(), Self::Error> {
        StorableGroupDataRef::new(group_id, GroupDataType::MessageSecrets, message_secrets)
            .store(self.connection)?;
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
        StorableGroupDataRef::new(
            group_id,
            GroupDataType::ResumptionPskStore,
            resumption_psk_store,
        )
        .store(self.connection)?;
        Ok(())
    }

    fn write_own_leaf_index<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        LeafNodeIndex: openmls_traits::storage::traits::LeafNodeIndex<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        own_leaf_index: &LeafNodeIndex,
    ) -> Result<(), Self::Error> {
        StorableGroupDataRef::new(group_id, GroupDataType::OwnLeafIndex, own_leaf_index)
            .store(self.connection)?;
        Ok(())
    }

    fn set_use_ratchet_tree_extension<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        value: bool,
    ) -> Result<(), Self::Error> {
        StorableGroupDataRef::new(group_id, GroupDataType::UseRatchetTreeExtension, &value)
            .store(self.connection)?;
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
        StorableGroupDataRef::new(
            group_id,
            GroupDataType::GroupEpochSecrets,
            group_epoch_secrets,
        )
        .store(self.connection)?;
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
        StorableSignatureKeyPairs::new(signature_key_pair)?.store(self.connection, public_key)
    }

    fn write_encryption_key_pair<
        EncryptionKey: openmls_traits::storage::traits::EncryptionKey<CURRENT_VERSION>,
        HpkeKeyPair: openmls_traits::storage::traits::HpkeKeyPair<CURRENT_VERSION>,
    >(
        &self,
        public_key: &EncryptionKey,
        key_pair: &HpkeKeyPair,
    ) -> Result<(), Self::Error> {
        StorableEncryptionKeyPair::new(key_pair)?.store(self.connection, public_key)
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
        StorableEpochKeyPairs::new(key_pairs)?.store(self.connection, group_id, epoch, leaf_index)
    }

    fn write_key_package<
        HashReference: openmls_traits::storage::traits::HashReference<CURRENT_VERSION>,
        KeyPackage: openmls_traits::storage::traits::KeyPackage<CURRENT_VERSION>,
    >(
        &self,
        hash_ref: &HashReference,
        key_package: &KeyPackage,
    ) -> Result<(), Self::Error> {
        key_package.store(self.connection, hash_ref)
    }

    fn write_psk<
        PskId: openmls_traits::storage::traits::PskId<CURRENT_VERSION>,
        PskBundle: openmls_traits::storage::traits::PskBundle<CURRENT_VERSION>,
    >(
        &self,
        psk_id: &PskId,
        psk: &PskBundle,
    ) -> Result<(), Self::Error> {
        StorablePskBundle::store(self.connection, psk_id, psk)
    }

    fn mls_group_join_config<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        MlsGroupJoinConfig: openmls_traits::storage::traits::MlsGroupJoinConfig<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<MlsGroupJoinConfig>, Self::Error> {
        let payload =
            StorableGroupData::load(self.connection, group_id, GroupDataType::JoinGroupConfig)?
                .map(|data| data.into_payload());
        Ok(payload)
    }

    fn own_leaf_nodes<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        LeafNode: openmls_traits::storage::traits::LeafNode<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Vec<LeafNode>, Self::Error> {
        let own_leaf_nodes = OwnLeafNode::load(self.connection, group_id)?
            .into_iter()
            .map(|node| node.into_inner())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(own_leaf_nodes)
    }

    fn aad<GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>>(
        &self,
        group_id: &GroupId,
    ) -> Result<Vec<u8>, Self::Error> {
        let payload = StorableGroupData::load(self.connection, group_id, GroupDataType::Aad)?
            .map(|data| data.into_payload())
            .unwrap_or_default();
        Ok(payload)
    }

    fn queued_proposal_refs<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        ProposalRef: openmls_traits::storage::traits::ProposalRef<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Vec<ProposalRef>, Self::Error> {
        let refs = StorableProposal::load_refs(self.connection, group_id)?;
        Ok(refs)
    }

    fn queued_proposals<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        ProposalRef: openmls_traits::storage::traits::ProposalRef<CURRENT_VERSION>,
        QueuedProposal: openmls_traits::storage::traits::QueuedProposal<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Vec<(ProposalRef, QueuedProposal)>, Self::Error> {
        let proposals = StorableProposal::load(self.connection, group_id)?
            .into_iter()
            .map(|p| p.into_tuple())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(proposals)
    }

    fn treesync<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        TreeSync: openmls_traits::storage::traits::TreeSync<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<TreeSync>, Self::Error> {
        let payload = StorableGroupData::load(self.connection, group_id, GroupDataType::Tree)?
            .map(|data| data.into_payload());
        Ok(payload)
    }

    fn group_context<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        GroupContext: openmls_traits::storage::traits::GroupContext<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<GroupContext>, Self::Error> {
        let payload = StorableGroupData::load(self.connection, group_id, GroupDataType::Context)?
            .map(|data| data.into_payload());
        Ok(payload)
    }

    fn interim_transcript_hash<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        InterimTranscriptHash: openmls_traits::storage::traits::InterimTranscriptHash<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<InterimTranscriptHash>, Self::Error> {
        let payload = StorableGroupData::load(
            self.connection,
            group_id,
            GroupDataType::InterimTranscriptHash,
        )?
        .map(|data| data.into_payload());
        Ok(payload)
    }

    fn confirmation_tag<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        ConfirmationTag: openmls_traits::storage::traits::ConfirmationTag<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<ConfirmationTag>, Self::Error> {
        let payload =
            StorableGroupData::load(self.connection, group_id, GroupDataType::ConfirmationTag)?
                .map(|data| data.into_payload());
        Ok(payload)
    }

    fn group_state<
        GroupState: openmls_traits::storage::traits::GroupState<CURRENT_VERSION>,
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<GroupState>, Self::Error> {
        let payload =
            StorableGroupData::load(self.connection, group_id, GroupDataType::GroupState)?
                .map(|data| data.into_payload());
        Ok(payload)
    }

    fn message_secrets<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        MessageSecrets: openmls_traits::storage::traits::MessageSecrets<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<MessageSecrets>, Self::Error> {
        let payload =
            StorableGroupData::load(self.connection, group_id, GroupDataType::MessageSecrets)?
                .map(|data| data.into_payload());
        Ok(payload)
    }

    fn resumption_psk_store<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        ResumptionPskStore: openmls_traits::storage::traits::ResumptionPskStore<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<ResumptionPskStore>, Self::Error> {
        let payload =
            StorableGroupData::load(self.connection, group_id, GroupDataType::ResumptionPskStore)?
                .map(|data| data.into_payload());
        Ok(payload)
    }

    fn own_leaf_index<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        LeafNodeIndex: openmls_traits::storage::traits::LeafNodeIndex<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<LeafNodeIndex>, Self::Error> {
        let payload =
            StorableGroupData::load(self.connection, group_id, GroupDataType::OwnLeafIndex)?
                .map(|data| data.into_payload());
        Ok(payload)
    }

    fn use_ratchet_tree_extension<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<bool>, Self::Error> {
        let payload = StorableGroupData::load(
            self.connection,
            group_id,
            GroupDataType::UseRatchetTreeExtension,
        )?
        .map(|data| data.into_payload());
        Ok(payload)
    }

    fn group_epoch_secrets<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        GroupEpochSecrets: openmls_traits::storage::traits::GroupEpochSecrets<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<GroupEpochSecrets>, Self::Error> {
        let payload =
            StorableGroupData::load(self.connection, group_id, GroupDataType::GroupEpochSecrets)?
                .map(|data| data.into_payload());
        Ok(payload)
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
        KeyPackage::load(self.connection, hash_ref)
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
        StorableProposal::delete(self.connection, group_id, proposal_ref)?;
        Ok(())
    }

    fn delete_aad<GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>>(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        StorableGroupData::<u8>::delete(self.connection, group_id, GroupDataType::Aad)?;
        Ok(())
    }

    fn delete_own_leaf_nodes<GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>>(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        OwnLeafNode::delete(self.connection, group_id)?;
        Ok(())
    }

    fn delete_group_config<GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>>(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        StorableGroupData::<u8>::delete(self.connection, group_id, GroupDataType::JoinGroupConfig)?;
        Ok(())
    }

    fn delete_tree<GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>>(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        StorableGroupData::<u8>::delete(self.connection, group_id, GroupDataType::Tree)?;
        Ok(())
    }

    fn delete_confirmation_tag<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        StorableGroupData::<u8>::delete(self.connection, group_id, GroupDataType::ConfirmationTag)?;
        Ok(())
    }

    fn delete_group_state<GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>>(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        StorableGroupData::<u8>::delete(self.connection, group_id, GroupDataType::GroupState)?;
        Ok(())
    }

    fn delete_context<GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>>(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        StorableGroupData::<u8>::delete(self.connection, group_id, GroupDataType::Context)?;
        Ok(())
    }

    fn delete_interim_transcript_hash<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        StorableGroupData::<u8>::delete(
            self.connection,
            group_id,
            GroupDataType::InterimTranscriptHash,
        )?;
        Ok(())
    }

    fn delete_message_secrets<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        StorableGroupData::<u8>::delete(self.connection, group_id, GroupDataType::MessageSecrets)?;
        Ok(())
    }

    fn delete_all_resumption_psk_secrets<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        StorableGroupData::<u8>::delete(
            self.connection,
            group_id,
            GroupDataType::ResumptionPskStore,
        )?;
        Ok(())
    }

    fn delete_own_leaf_index<GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>>(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        StorableGroupData::<u8>::delete(self.connection, group_id, GroupDataType::OwnLeafIndex)?;
        Ok(())
    }

    fn delete_use_ratchet_tree_extension<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        StorableGroupData::<u8>::delete(
            self.connection,
            group_id,
            GroupDataType::UseRatchetTreeExtension,
        )?;
        Ok(())
    }

    fn delete_group_epoch_secrets<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        StorableGroupData::<u8>::delete(
            self.connection,
            group_id,
            GroupDataType::GroupEpochSecrets,
        )?;
        Ok(())
    }

    fn clear_proposal_queue<
        GroupId: openmls_traits::storage::traits::GroupId<CURRENT_VERSION>,
        ProposalRef: openmls_traits::storage::traits::ProposalRef<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        StorableProposal::delete_all(self.connection, group_id)?;
        Ok(())
    }

    fn delete_signature_key_pair<
        SignaturePublicKey: openmls_traits::storage::traits::SignaturePublicKey<CURRENT_VERSION>,
    >(
        &self,
        public_key: &SignaturePublicKey,
    ) -> Result<(), Self::Error> {
        StorableSignatureKeyPairs::delete(self.connection, public_key)
    }

    fn delete_encryption_key_pair<
        EncryptionKey: openmls_traits::storage::traits::EncryptionKey<CURRENT_VERSION>,
    >(
        &self,
        public_key: &EncryptionKey,
    ) -> Result<(), Self::Error> {
        StorableEncryptionKeyPair::delete(self.connection, public_key)
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
        StorableEpochKeyPairs::delete(self.connection, group_id, epoch, leaf_index)
    }

    fn delete_key_package<
        KeyPackageRef: openmls_traits::storage::traits::HashReference<CURRENT_VERSION>,
    >(
        &self,
        hash_ref: &KeyPackageRef,
    ) -> Result<(), Self::Error> {
        KeyPackageBundle::delete(self.connection, hash_ref)
    }

    fn delete_psk<PskKey: openmls_traits::storage::traits::PskId<CURRENT_VERSION>>(
        &self,
        psk_id: &PskKey,
    ) -> Result<(), Self::Error> {
        StorablePskBundle::delete(self.connection, psk_id)
    }
}

#[derive(Serialize, Deserialize)]
struct Aad(Vec<u8>);

impl Entity<CURRENT_VERSION> for Aad {}
