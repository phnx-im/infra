// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use rusqlite::Connection;

use crate::utils::persistence::{DataType, Persistable, PersistableStruct, PersistenceError};

use super::*;

// TODO: When removing this, re-instate the foreign key constraints for GroupMembership!
pub(crate) struct GroupStore<'a> {
    db_connection: &'a Connection,
}

impl<'a> From<&'a Connection> for GroupStore<'a> {
    fn from(db_connection: &'a Connection) -> Self {
        Self { db_connection }
    }
}

impl<'a> GroupStore<'a> {
    pub(crate) fn get(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<PersistableGroup>, PersistenceError> {
        PersistableGroup::load_one(&self.db_connection, Some(&group_id), None)
    }

    pub(crate) fn create_group(
        &self,
        provider: &impl OpenMlsProvider,
        signer: &ClientSigningKey,
        group_id: GroupId,
        group_data: GroupData,
    ) -> Result<(PersistableGroup, PartialCreateGroupParams)> {
        let (payload, params) =
            Group::create_group(provider, &self.db_connection, signer, group_id, group_data)?;
        let group = PersistableGroup::from_connection_and_payload(&self.db_connection, payload);
        group.persist()?;
        Ok((group, params))
    }

    pub(crate) async fn join_group(
        &self,
        provider: &impl OpenMlsProvider<KeyStoreProvider = PhnxOpenMlsProvider<'a>>,
        welcome_bundle: WelcomeBundle,
        // This is our own key that the sender uses to encrypt to us. We should
        // be able to retrieve it from the client's key store.
        welcome_attribution_info_ear_key: &WelcomeAttributionInfoEarKey,
        leaf_key_store: LeafKeyStore<'_>,
        as_credential_store: AsCredentialStore<'_>,
        contact_store: ContactStore<'_>,
    ) -> Result<PersistableGroup> {
        let payload = Group::join_group(
            provider,
            welcome_bundle,
            welcome_attribution_info_ear_key,
            &self.db_connection,
            leaf_key_store,
            as_credential_store,
            contact_store,
        )
        .await?;
        let group = PersistableGroup::from_connection_and_payload(self.db_connection, payload);
        group.persist()?;
        Ok(group)
    }

    pub(crate) async fn join_group_externally(
        &self,
        provider: &impl OpenMlsProvider<KeyStoreProvider = PhnxOpenMlsProvider<'a>>,
        external_commit_info: ExternalCommitInfoIn,
        leaf_signer: InfraCredentialSigningKey,
        signature_ear_key: SignatureEarKey,
        group_state_ear_key: GroupStateEarKey,
        signature_ear_key_wrapper_key: SignatureEarKeyWrapperKey,
        credential_ear_key: ClientCredentialEarKey,
        as_credential_store: &AsCredentialStore<'_>,
        aad: InfraAadMessage,
        own_client_credential: &ClientCredential,
    ) -> Result<(PersistableGroup, MlsMessageOut, MlsMessageOut)> {
        let (payload, mls_message_out, mls_message_out_option) = Group::join_group_externally(
            provider,
            self.db_connection,
            external_commit_info,
            leaf_signer,
            signature_ear_key,
            group_state_ear_key,
            signature_ear_key_wrapper_key,
            credential_ear_key,
            as_credential_store,
            aad,
            own_client_credential,
        )
        .await?;
        let group = PersistableGroup::from_connection_and_payload(self.db_connection, payload);
        group.persist()?;
        Ok((group, mls_message_out, mls_message_out_option))
    }
}

pub(crate) type PersistableGroup<'a> = PersistableStruct<'a, Group>;

impl PersistableGroup<'_> {
    pub(crate) fn invite<'a>(
        &mut self,
        provider: &impl OpenMlsProvider<KeyStoreProvider = PhnxOpenMlsProvider<'a>>,
        signer: &ClientSigningKey,
        // The following three vectors have to be in sync, i.e. of the same length
        // and refer to the same contacts in order.
        add_infos: Vec<ContactAddInfos>,
        wai_keys: Vec<WelcomeAttributionInfoEarKey>,
        client_credentials: Vec<Vec<ClientCredential>>,
    ) -> Result<AddUsersParamsOut> {
        let result = self.payload.invite(
            provider,
            self.connection,
            signer,
            add_infos,
            wai_keys,
            client_credentials,
        )?;
        self.persist()?;
        Ok(result)
    }

    pub(crate) fn merge_pending_commit<'a>(
        &mut self,
        provider: &impl OpenMlsProvider<KeyStoreProvider = PhnxOpenMlsProvider<'a>>,
        staged_commit_option: impl Into<Option<StagedCommit>>,
        ds_timestamp: TimeStamp,
    ) -> Result<Vec<TimestampedMessage>> {
        let result = self.payload.merge_pending_commit(
            provider,
            self.connection,
            staged_commit_option,
            ds_timestamp,
        )?;
        self.persist()?;
        Ok(result)
    }

    pub(crate) fn remove<'a>(
        &mut self,
        provider: &impl OpenMlsProvider<KeyStoreProvider = PhnxOpenMlsProvider<'a>>,
        members: Vec<AsClientId>,
    ) -> Result<RemoveUsersParamsOut> {
        let result = self.payload.remove(provider, self.connection, members)?;
        self.persist()?;
        Ok(result)
    }

    pub(crate) fn create_message<'a>(
        &mut self,
        provider: &impl OpenMlsProvider<KeyStoreProvider = PhnxOpenMlsProvider<'a>>,
        content: MimiContent,
    ) -> Result<SendMessageParamsOut, GroupOperationError> {
        let result = self.payload.create_message(provider, content)?;
        self.persist()?;
        Ok(result)
    }

    pub(crate) async fn process_message<'a>(
        &mut self,
        provider: &impl OpenMlsProvider<KeyStoreProvider = PhnxOpenMlsProvider<'a>>,
        message: impl Into<ProtocolMessage>,
        as_credential_store: &AsCredentialStore<'_>,
    ) -> Result<(ProcessedMessage, bool, AsClientId)> {
        let result = self
            .payload
            .process_message(provider, self.connection, message, as_credential_store)
            .await?;
        self.persist()?;
        Ok(result)
    }

    pub(crate) fn store_proposal(&mut self, proposal: QueuedProposal) -> Result<()> {
        self.payload.store_proposal(proposal)?;
        self.persist()?;
        Ok(())
    }

    pub(crate) fn update_user_key(
        &mut self,
        provider: &impl OpenMlsProvider,
    ) -> Result<UpdateClientParamsOut> {
        let result = self.payload.update_user_key(provider, self.connection)?;
        self.persist()?;
        Ok(result)
    }

    pub(crate) fn delete<'a>(
        &mut self,
        provider: &impl OpenMlsProvider<KeyStoreProvider = PhnxOpenMlsProvider<'a>>,
    ) -> Result<DeleteGroupParamsOut> {
        let result = self.payload.delete(provider, self.connection)?;
        self.persist()?;
        Ok(result)
    }

    pub(crate) fn leave_group(
        &mut self,
        provider: &impl OpenMlsProvider,
    ) -> Result<SelfRemoveClientParamsOut> {
        let result = self.payload.leave_group(provider)?;
        self.persist()?;
        Ok(result)
    }

    pub(crate) fn update(
        &mut self,
        provider: &impl OpenMlsProvider,
    ) -> Result<UpdateClientParamsOut> {
        let result = self.payload.update(provider, self.connection)?;
        self.persist()?;
        Ok(result)
    }
}

impl Persistable for Group {
    type Key = GroupId;
    type SecondaryKey = GroupId;

    const DATA_TYPE: DataType = DataType::MlsGroup;

    fn key(&self) -> &Self::Key {
        &self.group_id
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &self.group_id
    }
}
