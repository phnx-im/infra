// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls_traits::storage::{Entity, Key, CURRENT_VERSION};
use rusqlite::{params, types::FromSql, Connection, OptionalExtension, ToSql};

use crate::utils::persistence::Storable;

use super::storage_provider::{EntityRefWrapper, EntityWrapper, KeyRefWrapper, StorableGroupIdRef};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GroupDataType {
    JoinGroupConfig,
    Tree,
    InterimTranscriptHash,
    Context,
    ConfirmationTag,
    GroupState,
    MessageSecrets,
    ResumptionPskStore,
    OwnLeafIndex,
    UseRatchetTreeExtension,
    GroupEpochSecrets,
}

impl ToSql for GroupDataType {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        match self {
            GroupDataType::JoinGroupConfig => "join_group_config".to_sql(),
            GroupDataType::Tree => "tree".to_sql(),
            GroupDataType::InterimTranscriptHash => "interim_transcript_hash".to_sql(),
            GroupDataType::Context => "context".to_sql(),
            GroupDataType::ConfirmationTag => "confirmation_tag".to_sql(),
            GroupDataType::GroupState => "group_state".to_sql(),
            GroupDataType::MessageSecrets => "message_secrets".to_sql(),
            GroupDataType::ResumptionPskStore => "resumption_psk_store".to_sql(),
            GroupDataType::OwnLeafIndex => "own_leaf_index".to_sql(),
            GroupDataType::UseRatchetTreeExtension => "use_ratchet_tree_extension".to_sql(),
            GroupDataType::GroupEpochSecrets => "group_epoch_secrets".to_sql(),
        }
    }
}

impl FromSql for GroupDataType {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let value = String::column_result(value)?;
        match value.as_str() {
            "join_group_config" => Ok(GroupDataType::JoinGroupConfig),
            "tree" => Ok(GroupDataType::Tree),
            "interim_transcript_hash" => Ok(GroupDataType::InterimTranscriptHash),
            "context" => Ok(GroupDataType::Context),
            "confirmation_tag" => Ok(GroupDataType::ConfirmationTag),
            "group_state" => Ok(GroupDataType::GroupState),
            "message_secrets" => Ok(GroupDataType::MessageSecrets),
            "resumption_psk_store" => Ok(GroupDataType::ResumptionPskStore),
            "own_leaf_index" => Ok(GroupDataType::OwnLeafIndex),
            "use_ratchet_tree_extension" => Ok(GroupDataType::UseRatchetTreeExtension),
            "group_epoch_secrets" => Ok(GroupDataType::GroupEpochSecrets),
            _ => Err(rusqlite::types::FromSqlError::InvalidType),
        }
    }
}

pub(crate) struct StorableGroupData<GroupData: Entity<CURRENT_VERSION>>(pub GroupData);

impl<GroupData: Entity<CURRENT_VERSION>> Storable for StorableGroupData<GroupData> {
    const CREATE_TABLE_STATEMENT: &'static str = "
        CREATE TABLE IF NOT EXISTS group_data (
            group_id BLOB NOT NULL,
            data_type TEXT NOT NULL CHECK (data_type IN (
                'join_group_config', 
                'tree', 
                'interim_transcript_hash',
                'context', 
                'confirmation_tag', 
                'group_state', 
                'message_secrets', 
                'resumption_psk_store',
                'own_leaf_index',
                'use_ratchet_tree_extension',
                'group_epoch_secrets'
            )),
            content BLOB NOT NULL,
            PRIMARY KEY (group_id, data_type)
        );";

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        let EntityWrapper(payload) = row.get(0)?;
        Ok(Self(payload))
    }
}

pub(super) struct StorableGroupDataRef<'a, GroupData: Entity<CURRENT_VERSION>>(pub &'a GroupData);

impl<GroupData: Entity<CURRENT_VERSION>> StorableGroupData<GroupData> {
    pub(super) fn load<GroupId: Key<CURRENT_VERSION>>(
        connection: &Connection,
        group_id: &GroupId,
        data_type: GroupDataType,
    ) -> Result<Option<GroupData>, rusqlite::Error> {
        let mut stmt = connection
            .prepare("SELECT content FROM group_data WHERE group_id = ? AND data_type = ?")?;
        stmt.query_row(params![KeyRefWrapper(group_id), data_type], Self::from_row)
            .map(|x| x.0)
            .optional()
    }
}

impl<'a, GroupData: Entity<CURRENT_VERSION>> StorableGroupDataRef<'a, GroupData> {
    pub(super) fn store<GroupId: Key<CURRENT_VERSION>>(
        &self,
        connection: &Connection,
        group_id: &GroupId,
        data_type: GroupDataType,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "INSERT OR REPLACE INTO group_data (group_id, data_type, content) VALUES (?, ?, ?)",
            params![KeyRefWrapper(group_id), data_type, EntityRefWrapper(self.0)],
        )?;
        Ok(())
    }
}

impl<'a, GroupId: Key<CURRENT_VERSION>> StorableGroupIdRef<'a, GroupId> {
    pub(super) fn delete_group_data(
        &self,
        connection: &Connection,
        data_type: GroupDataType,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "DELETE FROM group_data WHERE group_id = ? AND data_type = ?",
            params![KeyRefWrapper(self.0), data_type],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use openmls::{
        group::{GroupId, MlsGroup},
        prelude::{
            BasicCredential, CredentialWithKey, LeafNodeIndex, SignaturePublicKey, SignatureScheme,
        },
    };
    use openmls_rust_crypto::OpenMlsRustCrypto;
    use openmls_traits::{
        crypto::OpenMlsCrypto,
        signatures::{Signer, SignerError},
        storage::StorageProvider,
        OpenMlsProvider,
    };
    use phnxtypes::crypto::signatures::keys::generate_signature_keypair;

    use crate::groups::openmls_provider::{
        PhnxOpenMlsProvider, StorableEncryptionKeyPair, StorableEpochKeyPairs, StorableKeyPackage,
        StorableLeafNode, StorableProposal, StorablePskBundle, StorableSignatureKeyPairs,
    };

    use super::*;

    #[test]
    fn sql_conversion() {
        let connection = rusqlite::Connection::open_in_memory().unwrap();

        // Create a simple table that stores the data type
        let mut stmt = connection
            .prepare(StorableGroupData::<LeafNodeIndex>::CREATE_TABLE_STATEMENT)
            .unwrap();

        stmt.execute([]).unwrap();

        let data_type = GroupDataType::OwnLeafIndex;
        let leaf_node_index = LeafNodeIndex::new(42);
        let group_id = GroupId::from_slice(&[0x42; 16]);

        StorableGroupDataRef(&leaf_node_index)
            .store(&connection, &group_id, data_type)
            .unwrap();

        let loaded_group_data =
            StorableGroupData::<LeafNodeIndex>::load(&connection, &group_id, data_type)
                .unwrap()
                .unwrap();

        assert_eq!(leaf_node_index, loaded_group_data);
    }

    #[test]
    fn storage_provider() {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        let provider = PhnxOpenMlsProvider::new(&connection);

        // Create the table
        let mut stmt = connection
            .prepare(StorableGroupData::<LeafNodeIndex>::CREATE_TABLE_STATEMENT)
            .unwrap();
        stmt.execute([]).unwrap();

        let own_leaf_index = LeafNodeIndex::new(42);
        let group_id = GroupId::from_slice(&[0x42; 16]);

        provider
            .storage()
            .write_own_leaf_index(&group_id, &own_leaf_index)
            .unwrap();

        let loaded_leaf_index = provider
            .storage()
            .own_leaf_index(&group_id)
            .unwrap()
            .unwrap();

        assert_eq!(own_leaf_index, loaded_leaf_index);
    }

    struct TestSigner {
        signing_key: Vec<u8>,
        _verifying_key: Vec<u8>,
    }

    impl Signer for TestSigner {
        fn sign(&self, payload: &[u8]) -> Result<Vec<u8>, SignerError> {
            let rust_crypto = OpenMlsRustCrypto::default();
            Ok(rust_crypto
                .crypto()
                .sign(SignatureScheme::ED25519, payload, &self.signing_key)
                .unwrap())
        }

        fn signature_scheme(&self) -> SignatureScheme {
            SignatureScheme::ED25519
        }
    }

    #[test]
    fn mls_group_loading() {
        let connection = &mut rusqlite::Connection::open_in_memory().unwrap();

        <StorableGroupData<u8> as Storable>::create_table(connection).unwrap();
        <StorableLeafNode<u8> as Storable>::create_table(connection).unwrap();
        <StorableProposal<u8, u8> as Storable>::create_table(connection).unwrap();
        <StorableSignatureKeyPairs<u8> as Storable>::create_table(connection).unwrap();
        <StorableEpochKeyPairs<u8> as Storable>::create_table(connection).unwrap();
        <StorableEncryptionKeyPair<u8> as Storable>::create_table(connection).unwrap();
        <StorableKeyPackage<u8> as Storable>::create_table(connection).unwrap();
        <StorablePskBundle<u8> as Storable>::create_table(connection).unwrap();

        let transaction = connection.transaction().unwrap();

        let provider = PhnxOpenMlsProvider::new(&transaction);

        let (signing_key, verifying_key) = generate_signature_keypair().unwrap();
        let signer = TestSigner {
            signing_key,
            _verifying_key: verifying_key.clone(),
        };
        let credential = BasicCredential::new(b"test".into());
        let credential_with_key = CredentialWithKey {
            credential: credential.into(),
            signature_key: SignaturePublicKey::from(verifying_key.as_slice()),
        };
        let group_id = GroupId::from_slice(&[0x42; 16]);
        let group = MlsGroup::builder()
            .with_group_id(group_id.clone())
            .build(&provider, &signer, credential_with_key)
            .unwrap();

        let _group = MlsGroup::load(provider.storage(), group.group_id()).unwrap();

        transaction.commit().unwrap();
    }
}
