// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ops::Deref;

use rusqlite::Connection;

use crate::utils::persistence::{DataType, Persistable, PersistenceError};

use super::*;

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
        let group_id_bytes = GroupIdBytes::from(group_id.clone());
        PersistableGroup::load_one(&self.db_connection, Some(&group_id_bytes), None)
    }

    pub(crate) fn create_group(
        &self,
        provider: &impl OpenMlsProvider,
        signer: &ClientSigningKey,
        group_id: GroupId,
    ) -> Result<(PersistableGroup, PartialCreateGroupParams)> {
        let (payload, params) = Group::create_group(provider, signer, group_id)?;
        let group = PersistableGroup::from_connection_and_payload(&self.db_connection, payload);
        group.persist()?;
        Ok((group, params))
    }

    pub(crate) async fn join_group(
        &self,
        provider: &impl OpenMlsProvider<KeyStoreProvider = PhnxOpenMlsProvider>,
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
        provider: &impl OpenMlsProvider<KeyStoreProvider = PhnxOpenMlsProvider>,
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

pub(crate) struct PersistableGroup<'a> {
    connection: &'a Connection,
    payload: Group,
}

impl PersistableGroup<'_> {
    pub(crate) fn invite(
        &mut self,
        provider: &impl OpenMlsProvider<KeyStoreProvider = PhnxOpenMlsProvider>,
        signer: &ClientSigningKey,
        // The following three vectors have to be in sync, i.e. of the same length
        // and refer to the same contacts in order.
        add_infos: Vec<ContactAddInfos>,
        wai_keys: Vec<WelcomeAttributionInfoEarKey>,
        client_credentials: Vec<Vec<ClientCredential>>,
    ) -> Result<AddUsersParamsOut> {
        let result =
            self.payload
                .invite(provider, signer, add_infos, wai_keys, client_credentials)?;
        self.persist()?;
        Ok(result)
    }

    pub(crate) fn merge_pending_commit(
        &mut self,
        provider: &impl OpenMlsProvider<KeyStoreProvider = PhnxOpenMlsProvider>,
        staged_commit_option: impl Into<Option<StagedCommit>>,
    ) -> Result<Vec<GroupMessage>> {
        let result = self
            .payload
            .merge_pending_commit(provider, staged_commit_option)?;
        self.persist()?;
        Ok(result)
    }

    pub(crate) fn remove(
        &mut self,
        provider: &impl OpenMlsProvider<KeyStoreProvider = PhnxOpenMlsProvider>,
        members: Vec<AsClientId>,
    ) -> Result<RemoveUsersParamsOut> {
        let result = self.payload.remove(provider, members)?;
        self.persist()?;
        Ok(result)
    }

    pub fn create_message(
        &mut self,
        provider: &impl OpenMlsProvider<KeyStoreProvider = PhnxOpenMlsProvider>,
        msg: MessageContentType,
    ) -> Result<(SendMessageParamsOut, GroupMessage), GroupOperationError> {
        let result = self.payload.create_message(provider, msg)?;
        self.persist()?;
        Ok(result)
    }

    pub(crate) async fn process_message(
        &mut self,
        provider: &impl OpenMlsProvider<KeyStoreProvider = PhnxOpenMlsProvider>,
        message: impl Into<ProtocolMessage>,
        as_credential_store: &AsCredentialStore<'_>,
    ) -> Result<(ProcessedMessage, bool, ClientCredential)> {
        let result = self
            .payload
            .process_message(provider, message, as_credential_store)
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
        let result = self.payload.update_user_key(provider)?;
        self.persist()?;
        Ok(result)
    }

    pub(crate) fn delete(
        &mut self,
        provider: &impl OpenMlsProvider<KeyStoreProvider = PhnxOpenMlsProvider>,
    ) -> Result<DeleteGroupParamsOut> {
        let result = self.payload.delete(provider)?;
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
        let result = self.payload.update(provider)?;
        self.persist()?;
        Ok(result)
    }
}

impl Deref for PersistableGroup<'_> {
    type Target = Group;

    fn deref(&self) -> &Self::Target {
        &self.payload
    }
}

impl<'a> Persistable<'a> for PersistableGroup<'a> {
    type Key = GroupIdBytes;
    type SecondaryKey = GroupIdBytes;

    const DATA_TYPE: DataType = DataType::MlsGroup;

    fn key(&self) -> &Self::Key {
        &self.payload.group_id_bytes
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &self.payload.group_id_bytes
    }

    type Payload = Group;

    fn connection(&self) -> &Connection {
        self.connection
    }

    fn payload(&self) -> &Self::Payload {
        &self.payload
    }

    fn from_connection_and_payload(conn: &'a Connection, payload: Self::Payload) -> Self {
        Self {
            connection: conn,
            payload,
        }
    }
}
