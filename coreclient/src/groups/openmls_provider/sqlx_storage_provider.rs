#![allow(dead_code)]

use std::{cell::RefCell, future::Future};

use openmls_traits::storage::{
    traits::{
        self, ProposalRef as ProposalRefTrait, SignaturePublicKey as SignaturePublicKeyTrait,
    },
    Entity, Key, StorageProvider, CURRENT_VERSION,
};
use phnxtypes::codec::PhnxCodec;
use sqlx::{
    encode::IsNull, error::BoxDynError, query, sqlite::SqliteTypeInfo, Database, Decode, Encode,
    Row, Sqlite, SqliteConnection, SqliteExecutor, Type,
};
use tokio_stream::StreamExt;

use super::{
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
    storage_provider::{
        EntityRefWrapper, EntitySliceWrapper, EntityVecWrapper, EntityWrapper, KeyRefWrapper,
        StorableGroupIdRef,
    },
};

pub(crate) struct SqlxStorageProvider<'a> {
    connection: RefCell<&'a mut SqliteConnection>,
}

impl<'a> SqlxStorageProvider<'a> {
    pub(crate) fn new(connection: &'a mut SqliteConnection) -> Self {
        Self {
            connection: RefCell::new(connection),
        }
    }
}

impl StorageProvider<CURRENT_VERSION> for SqlxStorageProvider<'_> {
    type Error = sqlx::Error;

    fn write_mls_join_config<
        GroupId: traits::GroupId<CURRENT_VERSION>,
        MlsGroupJoinConfig: traits::MlsGroupJoinConfig<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        config: &MlsGroupJoinConfig,
    ) -> Result<(), Self::Error> {
        let storable = StorableGroupDataRef(config);
        let mut connection = self.connection.borrow_mut();
        let task = storable.store_sqlx(&mut **connection, group_id, GroupDataType::JoinGroupConfig);
        block_async_in_place(task)
    }

    fn append_own_leaf_node<
        GroupId: traits::GroupId<CURRENT_VERSION>,
        LeafNode: traits::LeafNode<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        leaf_node: &LeafNode,
    ) -> Result<(), Self::Error> {
        let storable = StorableLeafNodeRef(leaf_node);
        let mut connection = self.connection.borrow_mut();
        let task = storable.store_sqlx(&mut **connection, group_id);
        block_async_in_place(task)
    }

    fn queue_proposal<
        GroupId: traits::GroupId<CURRENT_VERSION>,
        ProposalRef: traits::ProposalRef<CURRENT_VERSION>,
        QueuedProposal: traits::QueuedProposal<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        proposal_ref: &ProposalRef,
        proposal: &QueuedProposal,
    ) -> Result<(), Self::Error> {
        let storable = StorableProposalRef(proposal_ref, proposal);
        let mut connection = self.connection.borrow_mut();
        let task = storable.store_sqlx(&mut **connection, group_id);
        block_async_in_place(task)
    }

    fn write_tree<
        GroupId: traits::GroupId<CURRENT_VERSION>,
        TreeSync: traits::TreeSync<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        tree: &TreeSync,
    ) -> Result<(), Self::Error> {
        let storable = StorableGroupDataRef(tree);
        let mut connection = self.connection.borrow_mut();
        let task = storable.store_sqlx(&mut **connection, group_id, GroupDataType::Tree);
        block_async_in_place(task)
    }

    fn write_interim_transcript_hash<
        GroupId: traits::GroupId<CURRENT_VERSION>,
        InterimTranscriptHash: traits::InterimTranscriptHash<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        interim_transcript_hash: &InterimTranscriptHash,
    ) -> Result<(), Self::Error> {
        let storable = StorableGroupDataRef(interim_transcript_hash);
        let mut connection = self.connection.borrow_mut();
        let task = storable.store_sqlx(
            &mut **connection,
            group_id,
            GroupDataType::InterimTranscriptHash,
        );
        block_async_in_place(task)
    }

    fn write_context<
        GroupId: traits::GroupId<CURRENT_VERSION>,
        GroupContext: traits::GroupContext<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        group_context: &GroupContext,
    ) -> Result<(), Self::Error> {
        let storable = StorableGroupDataRef(group_context);
        let mut connection = self.connection.borrow_mut();
        let task = storable.store_sqlx(&mut **connection, group_id, GroupDataType::Context);
        block_async_in_place(task)
    }

    fn write_confirmation_tag<
        GroupId: traits::GroupId<CURRENT_VERSION>,
        ConfirmationTag: traits::ConfirmationTag<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        confirmation_tag: &ConfirmationTag,
    ) -> Result<(), Self::Error> {
        let storable = StorableGroupDataRef(confirmation_tag);
        let mut connection = self.connection.borrow_mut();
        let task = storable.store_sqlx(&mut **connection, group_id, GroupDataType::ConfirmationTag);
        block_async_in_place(task)
    }

    fn write_group_state<
        GroupState: traits::GroupState<CURRENT_VERSION>,
        GroupId: traits::GroupId<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        group_state: &GroupState,
    ) -> Result<(), Self::Error> {
        let storable = StorableGroupDataRef(group_state);
        let mut connection = self.connection.borrow_mut();
        let task = storable.store_sqlx(&mut **connection, group_id, GroupDataType::GroupState);
        block_async_in_place(task)
    }

    fn write_message_secrets<
        GroupId: traits::GroupId<CURRENT_VERSION>,
        MessageSecrets: traits::MessageSecrets<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        message_secrets: &MessageSecrets,
    ) -> Result<(), Self::Error> {
        let storable = StorableGroupDataRef(message_secrets);
        let mut connection = self.connection.borrow_mut();
        let task = storable.store_sqlx(&mut **connection, group_id, GroupDataType::MessageSecrets);
        block_async_in_place(task)
    }

    fn write_resumption_psk_store<
        GroupId: traits::GroupId<CURRENT_VERSION>,
        ResumptionPskStore: traits::ResumptionPskStore<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        resumption_psk_store: &ResumptionPskStore,
    ) -> Result<(), Self::Error> {
        let storable = StorableGroupDataRef(resumption_psk_store);
        let mut connection = self.connection.borrow_mut();
        let task = storable.store_sqlx(
            &mut **connection,
            group_id,
            GroupDataType::ResumptionPskStore,
        );
        block_async_in_place(task)
    }

    fn write_own_leaf_index<
        GroupId: traits::GroupId<CURRENT_VERSION>,
        LeafNodeIndex: traits::LeafNodeIndex<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        own_leaf_index: &LeafNodeIndex,
    ) -> Result<(), Self::Error> {
        let storable = StorableGroupDataRef(own_leaf_index);
        let mut connection = self.connection.borrow_mut();
        let task = storable.store_sqlx(&mut **connection, group_id, GroupDataType::OwnLeafIndex);
        block_async_in_place(task)
    }

    fn write_group_epoch_secrets<
        GroupId: traits::GroupId<CURRENT_VERSION>,
        GroupEpochSecrets: traits::GroupEpochSecrets<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        group_epoch_secrets: &GroupEpochSecrets,
    ) -> Result<(), Self::Error> {
        let storable = StorableGroupDataRef(group_epoch_secrets);
        let mut connection = self.connection.borrow_mut();
        let task = storable.store_sqlx(
            &mut **connection,
            group_id,
            GroupDataType::GroupEpochSecrets,
        );
        block_async_in_place(task)
    }

    fn write_signature_key_pair<
        SignaturePublicKey: traits::SignaturePublicKey<CURRENT_VERSION>,
        SignatureKeyPair: traits::SignatureKeyPair<CURRENT_VERSION>,
    >(
        &self,
        public_key: &SignaturePublicKey,
        signature_key_pair: &SignatureKeyPair,
    ) -> Result<(), Self::Error> {
        let storable = StorableSignatureKeyPairsRef(signature_key_pair);
        let mut connection = self.connection.borrow_mut();
        let task = storable.store_sqlx(&mut **connection, public_key);
        block_async_in_place(task)
    }

    fn write_encryption_key_pair<
        EncryptionKey: traits::EncryptionKey<CURRENT_VERSION>,
        HpkeKeyPair: traits::HpkeKeyPair<CURRENT_VERSION>,
    >(
        &self,
        public_key: &EncryptionKey,
        key_pair: &HpkeKeyPair,
    ) -> Result<(), Self::Error> {
        let storable = StorableSignatureKeyPairsRef(key_pair);
        let mut connection = self.connection.borrow_mut();
        let task = storable.store_sqlx(&mut **connection, public_key);
        block_async_in_place(task)
    }

    fn write_encryption_epoch_key_pairs<
        GroupId: traits::GroupId<CURRENT_VERSION>,
        EpochKey: traits::EpochKey<CURRENT_VERSION>,
        HpkeKeyPair: traits::HpkeKeyPair<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        epoch: &EpochKey,
        leaf_index: u32,
        key_pairs: &[HpkeKeyPair],
    ) -> Result<(), Self::Error> {
        let storable = StorableEpochKeyPairsRef(key_pairs);
        let mut connection = self.connection.borrow_mut();
        let task = storable.store_sqlx(&mut **connection, group_id, epoch, leaf_index);
        block_async_in_place(task)
    }

    fn write_key_package<
        HashReference: traits::HashReference<CURRENT_VERSION>,
        KeyPackage: traits::KeyPackage<CURRENT_VERSION>,
    >(
        &self,
        hash_ref: &HashReference,
        key_package: &KeyPackage,
    ) -> Result<(), Self::Error> {
        let storable = StorableKeyPackageRef(key_package);
        let mut connection = self.connection.borrow_mut();
        let task = storable.store_sqlx(&mut **connection, hash_ref);
        block_async_in_place(task)
    }

    fn write_psk<
        PskId: traits::PskId<CURRENT_VERSION>,
        PskBundle: traits::PskBundle<CURRENT_VERSION>,
    >(
        &self,
        psk_id: &PskId,
        psk: &PskBundle,
    ) -> Result<(), Self::Error> {
        let storable = StorablePskBundleRef(psk);
        let mut connection = self.connection.borrow_mut();
        let task = storable.store_sqlx(&mut **connection, psk_id);
        block_async_in_place(task)
    }

    fn mls_group_join_config<
        GroupId: traits::GroupId<CURRENT_VERSION>,
        MlsGroupJoinConfig: traits::MlsGroupJoinConfig<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<MlsGroupJoinConfig>, Self::Error> {
        let mut connection = self.connection.borrow_mut();
        let task = StorableGroupData::load_sqlx(
            &mut **connection,
            group_id,
            GroupDataType::JoinGroupConfig,
        );
        block_async_in_place(task)
    }

    fn own_leaf_nodes<
        GroupId: traits::GroupId<CURRENT_VERSION>,
        LeafNode: traits::LeafNode<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Vec<LeafNode>, Self::Error> {
        let mut connection = self.connection.borrow_mut();
        let task = StorableLeafNode::load_sqlx(&mut **connection, group_id);
        block_async_in_place(task)
    }

    fn queued_proposal_refs<
        GroupId: traits::GroupId<CURRENT_VERSION>,
        ProposalRef: traits::ProposalRef<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Vec<ProposalRef>, Self::Error> {
        let mut connection = self.connection.borrow_mut();
        let task = StorableProposal::<u8, ProposalRef>::load_refs_sqlx(&mut **connection, group_id);
        block_async_in_place(task)
    }

    fn queued_proposals<
        GroupId: traits::GroupId<CURRENT_VERSION>,
        ProposalRef: traits::ProposalRef<CURRENT_VERSION>,
        QueuedProposal: traits::QueuedProposal<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Vec<(ProposalRef, QueuedProposal)>, Self::Error> {
        let mut connection = self.connection.borrow_mut();
        let task = StorableProposal::load_sqlx(&mut **connection, group_id);
        block_async_in_place(task)
    }

    fn tree<
        GroupId: traits::GroupId<CURRENT_VERSION>,
        TreeSync: traits::TreeSync<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<TreeSync>, Self::Error> {
        let mut connection = self.connection.borrow_mut();
        let task = StorableGroupData::load_sqlx(&mut **connection, group_id, GroupDataType::Tree);
        block_async_in_place(task)
    }

    fn group_context<
        GroupId: traits::GroupId<CURRENT_VERSION>,
        GroupContext: traits::GroupContext<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<GroupContext>, Self::Error> {
        let mut connection = self.connection.borrow_mut();
        let task =
            StorableGroupData::load_sqlx(&mut **connection, group_id, GroupDataType::Context);
        block_async_in_place(task)
    }

    fn interim_transcript_hash<
        GroupId: traits::GroupId<CURRENT_VERSION>,
        InterimTranscriptHash: traits::InterimTranscriptHash<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<InterimTranscriptHash>, Self::Error> {
        let mut connection = self.connection.borrow_mut();
        let task = StorableGroupData::load_sqlx(
            &mut **connection,
            group_id,
            GroupDataType::InterimTranscriptHash,
        );
        block_async_in_place(task)
    }

    fn confirmation_tag<
        GroupId: traits::GroupId<CURRENT_VERSION>,
        ConfirmationTag: traits::ConfirmationTag<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<ConfirmationTag>, Self::Error> {
        let mut connection = self.connection.borrow_mut();
        let task = StorableGroupData::load_sqlx(
            &mut **connection,
            group_id,
            GroupDataType::ConfirmationTag,
        );
        block_async_in_place(task)
    }

    fn group_state<
        GroupState: traits::GroupState<CURRENT_VERSION>,
        GroupId: traits::GroupId<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<GroupState>, Self::Error> {
        let mut connection = self.connection.borrow_mut();
        let task =
            StorableGroupData::load_sqlx(&mut **connection, group_id, GroupDataType::GroupState);
        block_async_in_place(task)
    }

    fn message_secrets<
        GroupId: traits::GroupId<CURRENT_VERSION>,
        MessageSecrets: traits::MessageSecrets<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<MessageSecrets>, Self::Error> {
        let mut connection = self.connection.borrow_mut();
        let task = StorableGroupData::load_sqlx(
            &mut **connection,
            group_id,
            GroupDataType::MessageSecrets,
        );
        block_async_in_place(task)
    }

    fn resumption_psk_store<
        GroupId: traits::GroupId<CURRENT_VERSION>,
        ResumptionPskStore: traits::ResumptionPskStore<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<ResumptionPskStore>, Self::Error> {
        let mut connection = self.connection.borrow_mut();
        let task = StorableGroupData::load_sqlx(
            &mut **connection,
            group_id,
            GroupDataType::ResumptionPskStore,
        );
        block_async_in_place(task)
    }

    fn own_leaf_index<
        GroupId: traits::GroupId<CURRENT_VERSION>,
        LeafNodeIndex: traits::LeafNodeIndex<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<LeafNodeIndex>, Self::Error> {
        let mut connection = self.connection.borrow_mut();
        let task =
            StorableGroupData::load_sqlx(&mut **connection, group_id, GroupDataType::OwnLeafIndex);
        block_async_in_place(task)
    }

    fn group_epoch_secrets<
        GroupId: traits::GroupId<CURRENT_VERSION>,
        GroupEpochSecrets: traits::GroupEpochSecrets<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<GroupEpochSecrets>, Self::Error> {
        let mut connection = self.connection.borrow_mut();
        let task = StorableGroupData::load_sqlx(
            &mut **connection,
            group_id,
            GroupDataType::GroupEpochSecrets,
        );
        block_async_in_place(task)
    }

    fn signature_key_pair<
        SignaturePublicKey: traits::SignaturePublicKey<CURRENT_VERSION>,
        SignatureKeyPair: traits::SignatureKeyPair<CURRENT_VERSION>,
    >(
        &self,
        public_key: &SignaturePublicKey,
    ) -> Result<Option<SignatureKeyPair>, Self::Error> {
        let mut connection = self.connection.borrow_mut();
        let task = StorableSignatureKeyPairs::load_sqlx(&mut **connection, public_key);
        block_async_in_place(task)
    }

    fn encryption_key_pair<
        HpkeKeyPair: traits::HpkeKeyPair<CURRENT_VERSION>,
        EncryptionKey: traits::EncryptionKey<CURRENT_VERSION>,
    >(
        &self,
        public_key: &EncryptionKey,
    ) -> Result<Option<HpkeKeyPair>, Self::Error> {
        let mut connection = self.connection.borrow_mut();
        let task = StorableEncryptionKeyPair::load_sqlx(&mut **connection, public_key);
        block_async_in_place(task)
    }

    fn encryption_epoch_key_pairs<
        GroupId: traits::GroupId<CURRENT_VERSION>,
        EpochKey: traits::EpochKey<CURRENT_VERSION>,
        HpkeKeyPair: traits::HpkeKeyPair<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        epoch: &EpochKey,
        leaf_index: u32,
    ) -> Result<Vec<HpkeKeyPair>, Self::Error> {
        let mut connection = self.connection.borrow_mut();
        let task = StorableEpochKeyPairs::load_sqlx(&mut **connection, group_id, epoch, leaf_index);
        block_async_in_place(task)
    }

    fn key_package<
        KeyPackageRef: traits::HashReference<CURRENT_VERSION>,
        KeyPackage: traits::KeyPackage<CURRENT_VERSION>,
    >(
        &self,
        hash_ref: &KeyPackageRef,
    ) -> Result<Option<KeyPackage>, Self::Error> {
        let mut connection = self.connection.borrow_mut();
        let task = StorableKeyPackage::load_sqlx(&mut **connection, hash_ref);
        block_async_in_place(task)
    }

    fn psk<PskBundle: traits::PskBundle<CURRENT_VERSION>, PskId: traits::PskId<CURRENT_VERSION>>(
        &self,
        psk_id: &PskId,
    ) -> Result<Option<PskBundle>, Self::Error> {
        let mut connection = self.connection.borrow_mut();
        let task = StorablePskBundle::load_sqlx(&mut **connection, psk_id);
        block_async_in_place(task)
    }

    fn remove_proposal<
        GroupId: traits::GroupId<CURRENT_VERSION>,
        ProposalRef: traits::ProposalRef<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        proposal_ref: &ProposalRef,
    ) -> Result<(), Self::Error> {
        let mut connection = self.connection.borrow_mut();
        let storable = StorableGroupIdRef(group_id);
        let task = storable.delete_proposal_sqlx(&mut **connection, proposal_ref);
        block_async_in_place(task)
    }

    fn delete_own_leaf_nodes<GroupId: traits::GroupId<CURRENT_VERSION>>(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        let storable = StorableGroupIdRef(group_id);
        let mut connection = self.connection.borrow_mut();
        let task = storable.delete_leaf_nodes_sqlx(&mut **connection);
        block_async_in_place(task)
    }

    fn delete_group_config<GroupId: traits::GroupId<CURRENT_VERSION>>(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        let storable = StorableGroupIdRef(group_id);
        let mut connection = self.connection.borrow_mut();
        let task =
            storable.delete_group_data_sqlx(&mut **connection, GroupDataType::JoinGroupConfig);
        block_async_in_place(task)
    }

    fn delete_tree<GroupId: traits::GroupId<CURRENT_VERSION>>(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        let storable = StorableGroupIdRef(group_id);
        let mut connection = self.connection.borrow_mut();
        let task = storable.delete_group_data_sqlx(&mut **connection, GroupDataType::Tree);
        block_async_in_place(task)
    }

    fn delete_confirmation_tag<GroupId: traits::GroupId<CURRENT_VERSION>>(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        let storable = StorableGroupIdRef(group_id);
        let mut connection = self.connection.borrow_mut();
        let task =
            storable.delete_group_data_sqlx(&mut **connection, GroupDataType::ConfirmationTag);
        block_async_in_place(task)
    }

    fn delete_group_state<GroupId: traits::GroupId<CURRENT_VERSION>>(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        let storable = StorableGroupIdRef(group_id);
        let mut connection = self.connection.borrow_mut();
        let task = storable.delete_group_data_sqlx(&mut **connection, GroupDataType::GroupState);
        block_async_in_place(task)
    }

    fn delete_context<GroupId: traits::GroupId<CURRENT_VERSION>>(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        let storable = StorableGroupIdRef(group_id);
        let mut connection = self.connection.borrow_mut();
        let task = storable.delete_group_data_sqlx(&mut **connection, GroupDataType::Context);
        block_async_in_place(task)
    }

    fn delete_interim_transcript_hash<GroupId: traits::GroupId<CURRENT_VERSION>>(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        let storable = StorableGroupIdRef(group_id);
        let mut connection = self.connection.borrow_mut();
        let task = storable
            .delete_group_data_sqlx(&mut **connection, GroupDataType::InterimTranscriptHash);
        block_async_in_place(task)
    }

    fn delete_message_secrets<GroupId: traits::GroupId<CURRENT_VERSION>>(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        let storable = StorableGroupIdRef(group_id);
        let mut connection = self.connection.borrow_mut();
        let task =
            storable.delete_group_data_sqlx(&mut **connection, GroupDataType::MessageSecrets);
        block_async_in_place(task)
    }

    fn delete_all_resumption_psk_secrets<GroupId: traits::GroupId<CURRENT_VERSION>>(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        let storable = StorableGroupIdRef(group_id);
        let mut connection = self.connection.borrow_mut();
        let task =
            storable.delete_group_data_sqlx(&mut **connection, GroupDataType::ResumptionPskStore);
        block_async_in_place(task)
    }

    fn delete_own_leaf_index<GroupId: traits::GroupId<CURRENT_VERSION>>(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        let storable = StorableGroupIdRef(group_id);
        let mut connection = self.connection.borrow_mut();
        let task = storable.delete_group_data_sqlx(&mut **connection, GroupDataType::OwnLeafIndex);
        block_async_in_place(task)
    }

    fn delete_group_epoch_secrets<GroupId: traits::GroupId<CURRENT_VERSION>>(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        let storable = StorableGroupIdRef(group_id);
        let mut connection = self.connection.borrow_mut();
        let task =
            storable.delete_group_data_sqlx(&mut **connection, GroupDataType::GroupEpochSecrets);
        block_async_in_place(task)
    }

    fn clear_proposal_queue<
        GroupId: traits::GroupId<CURRENT_VERSION>,
        ProposalRef: traits::ProposalRef<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
    ) -> Result<(), Self::Error> {
        let storable = StorableGroupIdRef(group_id);
        let mut connection = self.connection.borrow_mut();
        let task = storable.delete_all_proposals_sqlx(&mut **connection);
        block_async_in_place(task)
    }

    fn delete_signature_key_pair<
        SignaturePublicKey: traits::SignaturePublicKey<CURRENT_VERSION>,
    >(
        &self,
        public_key: &SignaturePublicKey,
    ) -> Result<(), Self::Error> {
        let storable = StorableSignaturePublicKeyRef(public_key);
        let mut connection = self.connection.borrow_mut();
        let task = storable.delete_sqlx(&mut **connection);
        block_async_in_place(task)
    }

    fn delete_encryption_key_pair<EncryptionKey: traits::EncryptionKey<CURRENT_VERSION>>(
        &self,
        public_key: &EncryptionKey,
    ) -> Result<(), Self::Error> {
        let storable = StorableEncryptionPublicKeyRef(public_key);
        let mut connection = self.connection.borrow_mut();
        let task = storable.delete_sqlx(&mut **connection);
        block_async_in_place(task)
    }

    fn delete_encryption_epoch_key_pairs<
        GroupId: traits::GroupId<CURRENT_VERSION>,
        EpochKey: traits::EpochKey<CURRENT_VERSION>,
    >(
        &self,
        group_id: &GroupId,
        epoch: &EpochKey,
        leaf_index: u32,
    ) -> Result<(), Self::Error> {
        let storable = StorableGroupIdRef(group_id);
        let mut connection = self.connection.borrow_mut();
        let task = storable.delete_epoch_key_pair_sqlx(&mut **connection, epoch, leaf_index);
        block_async_in_place(task)
    }

    fn delete_key_package<KeyPackageRef: traits::HashReference<CURRENT_VERSION>>(
        &self,
        hash_ref: &KeyPackageRef,
    ) -> Result<(), Self::Error> {
        let storable = StorableHashRef(hash_ref);
        let mut connection = self.connection.borrow_mut();
        let task = storable.delete_key_package_sqlx(&mut **connection);
        block_async_in_place(task)
    }

    fn delete_psk<PskKey: traits::PskId<CURRENT_VERSION>>(
        &self,
        psk_id: &PskKey,
    ) -> Result<(), Self::Error> {
        let storable = StorablePskIdRef(psk_id);
        let mut connection = self.connection.borrow_mut();
        let task = storable.delete_sqlx(&mut **connection);
        block_async_in_place(task)
    }
}

impl<T: Key<CURRENT_VERSION>> Type<Sqlite> for KeyRefWrapper<'_, T> {
    fn type_info() -> SqliteTypeInfo {
        <Vec<u8> as Type<Sqlite>>::type_info()
    }
}

impl<'q, T: Key<CURRENT_VERSION>> Encode<'q, Sqlite> for KeyRefWrapper<'_, T> {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as sqlx::Database>::ArgumentBuffer<'q>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        let key_bytes = PhnxCodec::to_vec(&self.0)?;
        Encode::<Sqlite>::encode(key_bytes, buf)
    }
}

impl<T: Entity<CURRENT_VERSION>> Type<Sqlite> for EntityRefWrapper<'_, T> {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <Vec<u8> as Type<Sqlite>>::type_info()
    }
}

impl<T: Entity<CURRENT_VERSION>> Encode<'_, Sqlite> for EntityRefWrapper<'_, T> {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'_>,
    ) -> Result<IsNull, BoxDynError> {
        let entity_bytes = PhnxCodec::to_vec(&self.0)?;
        Encode::<Sqlite>::encode(entity_bytes, buf)
    }
}

impl<GroupData: Entity<CURRENT_VERSION>> StorableGroupDataRef<'_, GroupData> {
    pub(super) async fn store_sqlx<GroupId: Key<CURRENT_VERSION>>(
        &self,
        executor: impl SqliteExecutor<'_>,
        group_id: &GroupId,
        data_type: GroupDataType,
    ) -> sqlx::Result<()> {
        let group_id = KeyRefWrapper(group_id);
        let group_data = EntityRefWrapper(self.0);
        query!(
            "INSERT OR REPLACE INTO group_data (group_id, data_type, group_data) VALUES (?, ?, ?)",
            group_id,
            data_type,
            group_data,
        )
        .execute(executor)
        .await?;
        Ok(())
    }
}

impl<SignatureKeyPairs: Entity<CURRENT_VERSION>>
    StorableSignatureKeyPairsRef<'_, SignatureKeyPairs>
{
    async fn store_sqlx<SignaturePublicKey: Key<CURRENT_VERSION>>(
        &self,
        executor: impl SqliteExecutor<'_>,
        public_key: &SignaturePublicKey,
    ) -> sqlx::Result<()> {
        let public_key = KeyRefWrapper(public_key);
        let signature_key = EntityRefWrapper(self.0);
        query!(
            "INSERT INTO signature_keys (public_key, signature_key) VALUES (?1, ?2)",
            public_key,
            signature_key
        )
        .execute(executor)
        .await?;
        Ok(())
    }
}

impl<LeafNode: Entity<CURRENT_VERSION>> StorableLeafNodeRef<'_, LeafNode> {
    async fn store_sqlx<GroupId: Key<CURRENT_VERSION>>(
        &self,
        executor: impl SqliteExecutor<'_>,
        group_id: &GroupId,
    ) -> sqlx::Result<()> {
        let group_id = KeyRefWrapper(group_id);
        let entity = EntityRefWrapper(self.0);
        query!(
            "INSERT INTO own_leaf_nodes (group_id, leaf_node) VALUES (?1, ?2)",
            group_id,
            entity,
        )
        .execute(executor)
        .await?;
        Ok(())
    }
}

impl<T: Entity<CURRENT_VERSION>> Type<Sqlite> for EntitySliceWrapper<'_, T> {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <Vec<u8> as Type<Sqlite>>::type_info()
    }
}

impl<T: Entity<CURRENT_VERSION>> Encode<'_, Sqlite> for EntitySliceWrapper<'_, T> {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'_>,
    ) -> Result<IsNull, BoxDynError> {
        let entity_bytes = PhnxCodec::to_vec(&self.0)?;
        Encode::<Sqlite>::encode(entity_bytes, buf)
    }
}

impl<KeyPackage: Entity<CURRENT_VERSION>> StorableKeyPackageRef<'_, KeyPackage> {
    async fn store_sqlx<KeyPackageRef: Key<CURRENT_VERSION>>(
        &self,
        executor: impl SqliteExecutor<'_>,
        key_package_ref: &KeyPackageRef,
    ) -> sqlx::Result<()> {
        let key_package_ref = KeyRefWrapper(key_package_ref);
        let key_package = EntityRefWrapper(self.0);
        query!(
            "INSERT INTO key_packages (key_package_ref, key_package) VALUES (?1, ?2)",
            key_package_ref,
            key_package,
        )
        .execute(executor)
        .await?;
        Ok(())
    }
}

impl<EpochKeyPairs: Entity<CURRENT_VERSION>> StorableEpochKeyPairsRef<'_, EpochKeyPairs> {
    async fn store_sqlx<GroupId: Key<CURRENT_VERSION>, EpochKey: Key<CURRENT_VERSION>>(
        &self,
        executor: impl SqliteExecutor<'_>,
        group_id: &GroupId,
        epoch_id: &EpochKey,
        leaf_index: u32,
    ) -> sqlx::Result<()> {
        let group_id = KeyRefWrapper(group_id);
        let epoch_id = KeyRefWrapper(epoch_id);
        let entity = EntitySliceWrapper(self.0);
        query!(
            "INSERT INTO epoch_keys_pairs (group_id, epoch_id, leaf_index, key_pairs)
            VALUES (?1, ?2, ?3, ?4)",
            group_id,
            epoch_id,
            leaf_index,
            entity,
        )
        .execute(executor)
        .await?;
        Ok(())
    }
}

impl<PskBundle: Entity<CURRENT_VERSION>> StorablePskBundleRef<'_, PskBundle> {
    async fn store_sqlx<PskId: Key<CURRENT_VERSION>>(
        &self,
        executor: impl SqliteExecutor<'_>,
        psk_id: &PskId,
    ) -> sqlx::Result<()> {
        let psk_id = KeyRefWrapper(psk_id);
        let psk_bundle = EntityRefWrapper(self.0);
        query!(
            "INSERT INTO psks (psk_id, psk_bundle) VALUES (?1, ?2)",
            psk_id,
            psk_bundle,
        )
        .execute(executor)
        .await?;
        Ok(())
    }
}

impl<T: Entity<CURRENT_VERSION>> Type<Sqlite> for EntityWrapper<T> {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <Vec<u8> as Type<Sqlite>>::type_info()
    }
}

impl<T: Entity<CURRENT_VERSION>> Decode<'_, Sqlite> for EntityWrapper<T> {
    fn decode(value: <Sqlite as Database>::ValueRef<'_>) -> Result<Self, BoxDynError> {
        let bytes: &[u8] = Decode::<Sqlite>::decode(value)?;
        let entity = PhnxCodec::from_slice(bytes)?;
        Ok(Self(entity))
    }
}

impl<GroupData: Entity<CURRENT_VERSION>> StorableGroupData<GroupData> {
    async fn load_sqlx<GroupId: Key<CURRENT_VERSION>>(
        executor: impl SqliteExecutor<'_>,
        group_id: &GroupId,
        data_type: GroupDataType,
    ) -> sqlx::Result<Option<GroupData>> {
        sqlx::query("SELECT group_data FROM group_data WHERE group_id = ? AND data_type = ?")
            .bind(KeyRefWrapper(group_id))
            .bind(data_type)
            .fetch_optional(executor)
            .await?
            .map(|row| {
                let EntityWrapper(group_data) = row.try_get(0)?;
                Ok(group_data)
            })
            .transpose()
    }
}

impl<Proposal: Entity<CURRENT_VERSION>, ProposalRef: Entity<CURRENT_VERSION>>
    StorableProposalRef<'_, Proposal, ProposalRef>
{
    async fn store_sqlx<GroupId: Key<CURRENT_VERSION>>(
        &self,
        executor: impl SqliteExecutor<'_>,
        group_id: &GroupId,
    ) -> sqlx::Result<()> {
        let group_id = KeyRefWrapper(group_id);
        let proposal_ref = EntityRefWrapper(self.0);
        let proposal = EntityRefWrapper(self.1);
        query!(
            "INSERT INTO proposals (group_id, proposal_ref, proposal) VALUES (?1, ?2, ?3)",
            group_id,
            proposal_ref,
            proposal
        )
        .execute(executor)
        .await?;
        Ok(())
    }
}

impl<LeafNode: Entity<CURRENT_VERSION>> StorableLeafNode<LeafNode> {
    async fn load_sqlx<GroupId: Key<CURRENT_VERSION>>(
        executor: impl SqliteExecutor<'_>,
        group_id: &GroupId,
    ) -> sqlx::Result<Vec<LeafNode>> {
        sqlx::query("SELECT leaf_node FROM own_leaf_nodes WHERE group_id = ?")
            .bind(KeyRefWrapper(group_id))
            .fetch(executor)
            .map(|row| {
                let EntityWrapper(leaf_node) = row?.try_get(0)?;
                Ok(leaf_node)
            })
            .collect()
            .await
    }
}

impl<Proposal: Entity<CURRENT_VERSION>, ProposalRef: Entity<CURRENT_VERSION>>
    StorableProposal<Proposal, ProposalRef>
{
    async fn load_sqlx<GroupId: Key<CURRENT_VERSION>>(
        executor: impl SqliteExecutor<'_>,
        group_id: &GroupId,
    ) -> sqlx::Result<Vec<(ProposalRef, Proposal)>> {
        sqlx::query("SELECT proposal_ref, proposal FROM proposals WHERE group_id = ?1")
            .bind(KeyRefWrapper(group_id))
            .fetch(executor)
            .map(|row| {
                let row = row?;
                let EntityWrapper(proposal_ref) = row.try_get(0)?;
                let EntityWrapper(proposal) = row.try_get(1)?;
                Ok((proposal_ref, proposal))
            })
            .collect()
            .await
    }

    async fn load_refs_sqlx<GroupId: Key<CURRENT_VERSION>>(
        executor: impl SqliteExecutor<'_>,
        group_id: &GroupId,
    ) -> sqlx::Result<Vec<ProposalRef>> {
        sqlx::query("SELECT proposal_ref FROM proposals WHERE group_id = ?1")
            .bind(KeyRefWrapper(group_id))
            .fetch(executor)
            .map(|row| {
                let EntityWrapper(proposal_ref) = row?.try_get(0)?;
                Ok(proposal_ref)
            })
            .collect()
            .await
    }
}

impl<SignatureKeyPairs: Entity<CURRENT_VERSION>> StorableSignatureKeyPairs<SignatureKeyPairs> {
    async fn load_sqlx<SignaturePublicKey: SignaturePublicKeyTrait<CURRENT_VERSION>>(
        executor: impl SqliteExecutor<'_>,
        public_key: &SignaturePublicKey,
    ) -> sqlx::Result<Option<SignatureKeyPairs>> {
        sqlx::query("SELECT signature_key FROM signature_keys WHERE public_key = ?1")
            .bind(KeyRefWrapper(public_key))
            .fetch_optional(executor)
            .await?
            .map(|row| {
                let EntityWrapper(signature_key) = row.try_get(0)?;
                Ok(signature_key)
            })
            .transpose()
    }
}

impl<EncryptionKeyPair: Entity<CURRENT_VERSION>> StorableEncryptionKeyPair<EncryptionKeyPair> {
    async fn load_sqlx<EncryptionKey: Key<CURRENT_VERSION>>(
        executor: impl SqliteExecutor<'_>,
        public_key: &EncryptionKey,
    ) -> sqlx::Result<Option<EncryptionKeyPair>> {
        sqlx::query("SELECT key_pair FROM encryption_keys WHERE public_key = ?1")
            .bind(KeyRefWrapper(public_key))
            .fetch_optional(executor)
            .await?
            .map(|row| {
                let EntityWrapper(encryption_key_pair) = row.try_get(0)?;
                Ok(encryption_key_pair)
            })
            .transpose()
    }
}

impl<EpochKeyPairs: Entity<CURRENT_VERSION>> StorableEpochKeyPairs<EpochKeyPairs> {
    async fn load_sqlx<GroupId: Key<CURRENT_VERSION>, EpochKey: Key<CURRENT_VERSION>>(
        executor: impl SqliteExecutor<'_>,
        group_id: &GroupId,
        epoch_id: &EpochKey,
        leaf_index: u32,
    ) -> sqlx::Result<Vec<EpochKeyPairs>> {
        let group_id = KeyRefWrapper(group_id);
        let epoch_id = KeyRefWrapper(epoch_id);
        sqlx::query(
            "SELECT key_pairs FROM epoch_keys_pairs
            WHERE group_id = ?1 AND epoch_id = ?2 AND leaf_index = ?3",
        )
        .bind(group_id)
        .bind(epoch_id)
        .bind(leaf_index)
        .fetch_optional(executor)
        .await?
        .map(|row| {
            let EntityVecWrapper(key_pairs) = row.try_get(0)?;
            Ok(key_pairs)
        })
        .transpose()
        .map(|res| res.unwrap_or_default())
    }
}

impl<T: Entity<CURRENT_VERSION>> Type<Sqlite> for EntityVecWrapper<T> {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <Vec<u8> as Type<Sqlite>>::type_info()
    }
}

impl<T: Entity<CURRENT_VERSION>> Decode<'_, Sqlite> for EntityVecWrapper<T> {
    fn decode(value: <Sqlite as Database>::ValueRef<'_>) -> Result<Self, BoxDynError> {
        let bytes: &[u8] = Decode::<Sqlite>::decode(value)?;
        let entities = PhnxCodec::from_slice(bytes)?;
        Ok(Self(entities))
    }
}

impl<KeyPackage: Entity<CURRENT_VERSION>> StorableKeyPackage<KeyPackage> {
    async fn load_sqlx<KeyPackageRef: Key<CURRENT_VERSION>>(
        executor: impl SqliteExecutor<'_>,
        key_package_ref: &KeyPackageRef,
    ) -> sqlx::Result<Option<KeyPackage>> {
        sqlx::query("SELECT key_package FROM key_packages WHERE key_package_ref = ?1")
            .bind(KeyRefWrapper(key_package_ref))
            .fetch_optional(executor)
            .await?
            .map(|row| {
                let EntityWrapper(key_package) = row.try_get(0)?;
                Ok(key_package)
            })
            .transpose()
    }
}

impl<PskBundle: Entity<CURRENT_VERSION>> StorablePskBundle<PskBundle> {
    async fn load_sqlx<PskId: Key<CURRENT_VERSION>>(
        executor: impl SqliteExecutor<'_>,
        psk_id: &PskId,
    ) -> sqlx::Result<Option<PskBundle>> {
        sqlx::query("SELECT psk_bundle FROM psks WHERE psk_id = ?1")
            .bind(KeyRefWrapper(psk_id))
            .fetch_optional(executor)
            .await?
            .map(|row| {
                let EntityWrapper(psk) = row.try_get(0)?;
                Ok(psk)
            })
            .transpose()
    }
}

impl<GroupId: Key<CURRENT_VERSION>> StorableGroupIdRef<'_, GroupId> {
    async fn delete_all_proposals_sqlx(
        &self,
        executor: impl SqliteExecutor<'_>,
    ) -> sqlx::Result<()> {
        let group_id = KeyRefWrapper(self.0);
        query!("DELETE FROM proposals WHERE group_id = ?1", group_id)
            .execute(executor)
            .await?;
        Ok(())
    }

    async fn delete_proposal_sqlx<ProposalRef: ProposalRefTrait<CURRENT_VERSION>>(
        &self,
        executor: impl SqliteExecutor<'_>,
        proposal_ref: &ProposalRef,
    ) -> sqlx::Result<()> {
        let group_id = KeyRefWrapper(self.0);
        let proposal_ref = KeyRefWrapper(proposal_ref);
        query!(
            "DELETE FROM proposals WHERE group_id = ?1 AND proposal_ref = ?2",
            group_id,
            proposal_ref,
        )
        .execute(executor)
        .await?;
        Ok(())
    }

    async fn delete_leaf_nodes_sqlx(&self, executor: impl SqliteExecutor<'_>) -> sqlx::Result<()> {
        let group_id = KeyRefWrapper(self.0);
        query!("DELETE FROM own_leaf_nodes WHERE group_id = ?1", group_id)
            .execute(executor)
            .await?;
        Ok(())
    }

    async fn delete_group_data_sqlx(
        &self,
        executor: impl SqliteExecutor<'_>,
        data_type: GroupDataType,
    ) -> sqlx::Result<()> {
        let group_id = KeyRefWrapper(self.0);
        query!(
            "DELETE FROM group_data WHERE group_id = ? AND data_type = ?",
            group_id,
            data_type
        )
        .execute(executor)
        .await?;
        Ok(())
    }

    async fn delete_epoch_key_pair_sqlx<EpochKey: Key<CURRENT_VERSION>>(
        &self,
        executor: impl SqliteExecutor<'_>,
        epoch_key: &EpochKey,
        leaf_index: u32,
    ) -> sqlx::Result<()> {
        let group_id = KeyRefWrapper(self.0);
        let epoch_key = KeyRefWrapper(epoch_key);
        query!(
            "DELETE FROM epoch_keys_pairs WHERE group_id = ? AND epoch_id = ? AND leaf_index = ?",
            group_id,
            epoch_key,
            leaf_index,
        )
        .execute(executor)
        .await?;
        Ok(())
    }
}

impl<SignaturePublicKey: Key<CURRENT_VERSION>>
    StorableSignaturePublicKeyRef<'_, SignaturePublicKey>
{
    async fn delete_sqlx(&self, executor: impl SqliteExecutor<'_>) -> sqlx::Result<()> {
        let public_key = KeyRefWrapper(self.0);
        query!(
            "DELETE FROM signature_keys WHERE public_key = ?1",
            public_key
        )
        .execute(executor)
        .await?;
        Ok(())
    }
}

impl<EncryptionPublicKey: Key<CURRENT_VERSION>>
    StorableEncryptionPublicKeyRef<'_, EncryptionPublicKey>
{
    async fn delete_sqlx(&self, executor: impl SqliteExecutor<'_>) -> sqlx::Result<()> {
        let public_key = KeyRefWrapper(self.0);
        query!(
            "DELETE FROM encryption_keys WHERE public_key = ?1",
            public_key
        )
        .execute(executor)
        .await?;
        Ok(())
    }
}

impl<KeyPackageRef: Key<CURRENT_VERSION>> StorableHashRef<'_, KeyPackageRef> {
    async fn delete_key_package_sqlx(&self, executor: impl SqliteExecutor<'_>) -> sqlx::Result<()> {
        let hash_ref = KeyRefWrapper(self.0);
        query!(
            "DELETE FROM key_packages WHERE key_package_ref = ?1",
            hash_ref,
        )
        .execute(executor)
        .await?;
        Ok(())
    }
}

impl<PskId: Key<CURRENT_VERSION>> StorablePskIdRef<'_, PskId> {
    async fn delete_sqlx(&self, executor: impl SqliteExecutor<'_>) -> sqlx::Result<()> {
        let psks_id = KeyRefWrapper(self.0);
        query!("DELETE FROM psks WHERE psk_id = ?1", psks_id)
            .execute(executor)
            .await?;
        Ok(())
    }
}

/// Runs and waits for the given future to complete in a synchronous context.
///
/// Note that even though this function is called in a synchronous context, at some point down the
/// stack it must be called in a multi-threaded asynchronous context. In particular, tests must be
/// asynchronous and of flavor `multi_thread`.
fn block_async_in_place<F>(task: F) -> F::Output
where
    F: Future,
{
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(task))
}
