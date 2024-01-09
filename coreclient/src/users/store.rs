// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::bail;
use rusqlite::Transaction;

use crate::utils::persistence::{open_phnx_db, PersistableStruct, SqlKey};

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
    fn client_id(&self) -> &AsClientId {
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
    ) -> Result<Self> {
        let client_record = PersistableClientRecord::new(&phnx_db_connection, as_client_id.clone());
        client_record.persist()?;

        let basic_user_data = BasicUserData {
            as_client_id,
            server_url: server_url.to_string(),
            password: password.to_string(),
        };
        UserCreationState::BasicUserData(basic_user_data).persist(client_db_connection)
    }

    pub(super) async fn step(
        self,
        phnx_db_connection: &Connection,
        client_db_transaction: &mut Transaction<'_>,
        api_clients: &ApiClients,
    ) -> Result<Self> {
        // If we're already in the final state, there is nothing to do.
        if matches!(self, UserCreationState::FinalUserState(_)) {
            return Ok(self);
        }

        let savepoint = client_db_transaction.savepoint()?;

        let new_state = match self {
            UserCreationState::BasicUserData(state) => Self::InitialUserState(
                state
                    .prepare_as_registration(&savepoint, api_clients)
                    .await?,
            ),
            UserCreationState::InitialUserState(state) => {
                Self::PostRegistrationInitState(state.initiate_as_registration(api_clients).await?)
            }
            UserCreationState::PostRegistrationInitState(state) => {
                Self::UnfinalizedRegistrationState(state.process_server_response(&savepoint)?)
            }
            UserCreationState::UnfinalizedRegistrationState(state) => {
                Self::AsRegisteredUserState(state.finalize_as_registration(api_clients).await?)
            }
            UserCreationState::AsRegisteredUserState(state) => {
                Self::QsRegisteredUserState(state.register_with_qs(api_clients).await?)
            }
            UserCreationState::QsRegisteredUserState(state) => {
                Self::FinalUserState(state.upload_add_packages(&savepoint, api_clients).await?)
            }
            UserCreationState::FinalUserState(_) => self,
        }
        .persist(&savepoint)?;

        savepoint.commit()?;

        // If we just transitioned into the final state, we need to update the
        // client record.
        if let UserCreationState::FinalUserState(_) = new_state {
            let mut client_record = PersistableClientRecord::load_one(
                phnx_db_connection,
                Some(new_state.client_id()),
                None,
            )?
            .ok_or(anyhow!("Client record not found"))?;
            client_record.finish()?;
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
        phnx_db_connection: &Connection,
        client_db_transaction: &mut Transaction<'_>,
        api_clients: &ApiClients,
    ) -> Result<PersistedUserState> {
        while !matches!(self, UserCreationState::FinalUserState(_)) {
            self = self
                .step(phnx_db_connection, client_db_transaction, &api_clients)
                .await?
        }

        self.final_state()
    }

    fn persist(self, connection: &Connection) -> Result<Self> {
        let persistable_state = PersistableUserData::from_connection_and_payload(connection, self);
        persistable_state.persist()?;
        Ok(persistable_state.into_payload())
    }

    #[cfg(test)]
    pub(super) fn load(
        connection: &Connection,
        as_client_id: &AsClientId,
    ) -> Result<Option<Self>, PersistenceError> {
        PersistableUserData::load_one(connection, Some(as_client_id), None)
            .map(|persistable| persistable.map(|p| p.into_payload()))
    }
}

pub(super) type PersistableUserData<'a> = PersistableStruct<'a, UserCreationState>;

impl PersistableUserData<'_> {
    pub(super) fn into_payload(self) -> UserCreationState {
        self.payload
    }

    pub(super) fn server_url(&self) -> &str {
        self.payload.server_url()
    }
}

impl SqlKey for AsClientId {
    fn to_sql_key(&self) -> String {
        self.to_string()
    }
}

impl Persistable for UserCreationState {
    type Key = AsClientId;

    type SecondaryKey = AsClientId;

    const DATA_TYPE: DataType = DataType::ClientData;

    fn key(&self) -> &Self::Key {
        &self.client_id()
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &self.client_id()
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

pub(super) type PersistableClientRecord<'a> = PersistableStruct<'a, ClientRecord>;

impl<'a> PersistableClientRecord<'a> {
    pub(super) fn new(connection: &'a Connection, as_client_id: AsClientId) -> Self {
        Self {
            connection,
            payload: ClientRecord {
                as_client_id,
                client_record_state: ClientRecordState::InProgress,
            },
        }
    }

    pub(super) fn finish(&mut self) -> Result<(), PersistenceError> {
        self.payload.client_record_state = ClientRecordState::Finished;
        self.persist()
    }

    pub(super) fn into_payload(self) -> ClientRecord {
        self.payload
    }
}

impl ClientRecord {
    pub fn load_all(path: &str) -> Result<Vec<Self>, PersistenceError> {
        let connection = open_phnx_db(path)?;
        Self::load_all_from_db(&connection)
    }

    pub fn load_all_from_db(connection: &Connection) -> Result<Vec<Self>, PersistenceError> {
        let records = PersistableClientRecord::load_all(&connection)?
            .into_iter()
            .map(|record| record.into_payload())
            .collect();
        Ok(records)
    }
}

impl Persistable for ClientRecord {
    type Key = AsClientId;

    type SecondaryKey = AsClientId;

    const DATA_TYPE: DataType = DataType::ClientRecord;

    fn key(&self) -> &Self::Key {
        &self.as_client_id
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &self.as_client_id
    }
}
