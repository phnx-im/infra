// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::bail;
use groups::{
    openmls_provider::{
        StorableEncryptionKeyPair, StorableEpochKeyPairs, StorableGroupData, StorableKeyPackage,
        StorableLeafNode, StorableProposal, StorablePskBundle, StorableSignatureKeyPairs,
    },
    persistence::StorableGroup,
};
use key_stores::{
    qs_verifying_keys::StorableQsVerifyingKey, queue_ratchets::StorableAsQueueRatchet,
};
use own_client_info::OwnClientInfo;
use phnxtypes::messages::push_token::PushToken;

use self::{
    groups::client_auth_info::{GroupMembership, StorableClientCredential},
    key_stores::leaf_keys::LeafKeys,
    utils::persistence::{Storable, Triggerable},
};

use super::{
    create_user::{
        AsRegisteredUserState, BasicUserData, PersistedUserState, PostRegistrationInitState,
        QsRegisteredUserState, UnfinalizedRegistrationState,
    },
    *,
};

#[derive(Serialize, Deserialize)]
pub(super) enum UserCreationState {
    BasicUserData(BasicUserData),
    InitialUserState(InitialUserState),
    PostRegistrationInitState(PostRegistrationInitState),
    UnfinalizedRegistrationState(UnfinalizedRegistrationState),
    AsRegisteredUserState(AsRegisteredUserState),
    QsRegisteredUserState(QsRegisteredUserState),
    FinalUserState(PersistedUserState),
}

impl UserCreationState {
    pub(super) fn client_id(&self) -> &AsClientId {
        match self {
            Self::BasicUserData(state) => state.client_id(),
            Self::InitialUserState(state) => state.client_id(),
            Self::PostRegistrationInitState(state) => state.client_id(),
            Self::UnfinalizedRegistrationState(state) => state.client_id(),
            Self::AsRegisteredUserState(state) => state.client_id(),
            Self::QsRegisteredUserState(state) => state.client_id(),
            Self::FinalUserState(state) => state.client_id(),
        }
    }

    pub(super) fn server_url(&self) -> &str {
        match self {
            Self::BasicUserData(state) => state.server_url(),
            Self::InitialUserState(state) => state.server_url(),
            Self::PostRegistrationInitState(state) => state.server_url(),
            Self::UnfinalizedRegistrationState(state) => state.server_url(),
            Self::AsRegisteredUserState(state) => state.server_url(),
            Self::QsRegisteredUserState(state) => state.server_url(),
            Self::FinalUserState(state) => state.server_url(),
        }
    }

    pub(super) fn new(
        client_db_connection: &Connection,
        phnx_db_connection: &Connection,
        as_client_id: AsClientId,
        server_url: impl ToString,
        password: &str,
        push_token: Option<PushToken>,
    ) -> Result<Self> {
        let client_record = ClientRecord::new(as_client_id.clone());
        client_record.store(phnx_db_connection)?;

        let basic_user_data = BasicUserData {
            as_client_id: as_client_id.clone(),
            server_url: server_url.to_string(),
            password: password.to_string(),
            push_token,
        };

        // Create user profile entry for own user.
        UserProfile::store_own_user_profile(
            client_db_connection,
            as_client_id.user_name(),
            None,
            None,
        )?;

        let user_creation_state = UserCreationState::BasicUserData(basic_user_data);

        user_creation_state.store(client_db_connection)?;

        Ok(user_creation_state)
    }

    pub(super) async fn step(
        self,
        phnx_db_connection: SqliteConnection,
        client_db_connection: SqliteConnection,
        api_clients: &ApiClients,
    ) -> Result<Self> {
        // If we're already in the final state, there is nothing to do.
        if matches!(self, UserCreationState::FinalUserState(_)) {
            return Ok(self);
        }

        let new_state = match self {
            UserCreationState::BasicUserData(state) => Self::InitialUserState(
                state
                    .prepare_as_registration(client_db_connection.clone(), api_clients)
                    .await?,
            ),
            UserCreationState::InitialUserState(state) => {
                Self::PostRegistrationInitState(state.initiate_as_registration(api_clients).await?)
            }
            UserCreationState::PostRegistrationInitState(state) => {
                let connection = client_db_connection.lock().await;
                Self::UnfinalizedRegistrationState(state.process_server_response(&connection)?)
            }
            UserCreationState::UnfinalizedRegistrationState(state) => {
                Self::AsRegisteredUserState(state.finalize_as_registration(api_clients).await?)
            }
            UserCreationState::AsRegisteredUserState(state) => {
                Self::QsRegisteredUserState(state.register_with_qs(api_clients).await?)
            }
            UserCreationState::QsRegisteredUserState(state) => Self::FinalUserState(
                state
                    .upload_add_packages(client_db_connection.clone(), api_clients)
                    .await?,
            ),
            UserCreationState::FinalUserState(_) => self,
        };

        let client_db_connection = client_db_connection.lock().await;
        new_state.store(&client_db_connection)?;

        // If we just transitioned into the final state, we need to update the
        // client record.
        let phnx_db_connection = phnx_db_connection.lock().await;
        if let UserCreationState::FinalUserState(_) = new_state {
            let mut client_record = ClientRecord::load(&phnx_db_connection, new_state.client_id())?
                .ok_or(anyhow!("Client record not found"))?;
            client_record.finish();
            client_record.store(&phnx_db_connection)?;
        }

        Ok(new_state)
    }

    pub(super) fn final_state(self) -> Result<PersistedUserState> {
        if let UserCreationState::FinalUserState(state) = self {
            Ok(state)
        } else {
            bail!("Unexpected user creation state")
        }
    }

    /// A convenience function that performs the `step` function until the final state is reached.
    pub(super) async fn complete_user_creation(
        mut self,
        phnx_db_connection: SqliteConnection,
        client_db_connection: SqliteConnection,
        api_clients: &ApiClients,
    ) -> Result<PersistedUserState> {
        while !matches!(self, UserCreationState::FinalUserState(_)) {
            self = self
                .step(
                    phnx_db_connection.clone(),
                    client_db_connection.clone(),
                    api_clients,
                )
                .await?
        }

        self.final_state()
    }
}

#[derive(Serialize, Deserialize)]
pub enum ClientRecordState {
    InProgress,
    Finished,
}

#[derive(Serialize, Deserialize)]
pub struct ClientRecord {
    pub as_client_id: AsClientId,
    pub client_record_state: ClientRecordState,
}

impl ClientRecord {
    pub(super) fn new(as_client_id: AsClientId) -> Self {
        Self {
            as_client_id,
            client_record_state: ClientRecordState::InProgress,
        }
    }

    pub(super) fn finish(&mut self) {
        self.client_record_state = ClientRecordState::Finished;
    }
}

/// Create all tables for a client database by calling the `create_table`
/// function of all structs that implement `Persistable`.
pub(crate) fn create_all_tables(client_db_connection: &Connection) -> Result<(), rusqlite::Error> {
    <UserCreationState as Storable>::create_table(client_db_connection)?;
    <OwnClientInfo as Storable>::create_table(client_db_connection)?;
    <UserProfile as Storable>::create_table(client_db_connection)?;
    <StorableGroup as Storable>::create_table(client_db_connection)?;
    <StorableClientCredential as Storable>::create_table(client_db_connection)?;
    <GroupMembership as Storable>::create_table(client_db_connection)?;
    <Contact as Storable>::create_table(client_db_connection)?;
    <PartialContact as Storable>::create_table(client_db_connection)?;
    <Conversation as Storable>::create_table(client_db_connection)?;
    <ConversationMessage as Storable>::create_table(client_db_connection)?;
    <AsCredentials as Storable>::create_table(client_db_connection)?;
    <LeafKeys as Storable>::create_table(client_db_connection)?;
    <StorableQsVerifyingKey as Storable>::create_table(client_db_connection)?;
    // The table for queue ratchets contains both the AsQueueRatchet and the
    // QsQueueRatchet.
    <StorableAsQueueRatchet as Storable>::create_table(client_db_connection)?;

    // OpenMLS provider data
    <StorableGroupData<u8> as Storable>::create_table(client_db_connection)?;
    <StorableLeafNode<u8> as Storable>::create_table(client_db_connection)?;
    <StorableProposal<u8, u8> as Storable>::create_table(client_db_connection)?;
    <StorableSignatureKeyPairs<u8> as Storable>::create_table(client_db_connection)?;
    <StorableEpochKeyPairs<u8> as Storable>::create_table(client_db_connection)?;
    <StorableEncryptionKeyPair<u8> as Storable>::create_table(client_db_connection)?;
    <StorableKeyPackage<u8> as Storable>::create_table(client_db_connection)?;
    <StorablePskBundle<u8> as Storable>::create_table(client_db_connection)?;

    Ok(())
}

pub(crate) fn create_all_triggers(
    client_db_connection: &Connection,
) -> Result<(), rusqlite::Error> {
    <GroupMembership as Triggerable>::create_trigger(client_db_connection)?;
    <Contact as Triggerable>::create_trigger(client_db_connection)?;
    <PartialContact as Triggerable>::create_trigger(client_db_connection)?;

    Ok(())
}
