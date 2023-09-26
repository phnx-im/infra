// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::bail;

use super::{
    create_user::{
        AsRegisteredUserState, PersistedUserState, PostRegistrationInitState,
        QsRegisteredUserState, UnfinalizedRegistrationState,
    },
    *,
};

#[derive(Serialize, Deserialize)]
pub(super) enum UserCreationState {
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
            Self::InitialUserState(state) => state.server_url(),
            Self::PostRegistrationInitState(state) => state.server_url(),
            Self::UnfinalizedRegistrationState(state) => state.server_url(),
            Self::AsRegisteredUserState(state) => state.server_url(),
            Self::QsRegisteredUserState(state) => state.server_url(),
            Self::FinalUserState(state) => state.server_url(),
        }
    }
}

impl InitialUserState {
    pub(super) fn persist(self, connection: &Connection) -> Result<Self> {
        let user_data = PersistableUserData::from_connection_and_payload(
            connection,
            UserCreationState::InitialUserState(self),
        );
        user_data.persist()?;
        if let UserCreationState::InitialUserState(state) = user_data.into_payload() {
            Ok(state)
        } else {
            bail!("Unexpected user creation state")
        }
    }
}

impl PostRegistrationInitState {
    pub(super) fn persist(self, connection: &Connection) -> Result<Self> {
        let user_data = PersistableUserData::from_connection_and_payload(
            connection,
            UserCreationState::PostRegistrationInitState(self),
        );
        user_data.persist()?;
        if let UserCreationState::PostRegistrationInitState(state) = user_data.into_payload() {
            Ok(state)
        } else {
            bail!("Unexpected user creation state")
        }
    }
}

impl UnfinalizedRegistrationState {
    pub(super) fn persist(self, connection: &Connection) -> Result<Self> {
        let user_data = PersistableUserData::from_connection_and_payload(
            connection,
            UserCreationState::UnfinalizedRegistrationState(self),
        );
        user_data.persist()?;
        if let UserCreationState::UnfinalizedRegistrationState(state) = user_data.into_payload() {
            Ok(state)
        } else {
            bail!("Unexpected user creation state")
        }
    }
}

impl AsRegisteredUserState {
    pub(super) fn persist(self, connection: &Connection) -> Result<Self> {
        let user_data = PersistableUserData::from_connection_and_payload(
            connection,
            UserCreationState::AsRegisteredUserState(self),
        );
        user_data.persist()?;
        if let UserCreationState::AsRegisteredUserState(state) = user_data.into_payload() {
            Ok(state)
        } else {
            bail!("Unexpected user creation state")
        }
    }
}

impl QsRegisteredUserState {
    pub(super) fn persist(self, connection: &Connection) -> Result<Self> {
        let user_data = PersistableUserData::from_connection_and_payload(
            connection,
            UserCreationState::QsRegisteredUserState(self),
        );
        user_data.persist()?;
        if let UserCreationState::QsRegisteredUserState(state) = user_data.into_payload() {
            Ok(state)
        } else {
            bail!("Unexpected user creation state")
        }
    }
}

impl PersistedUserState {
    pub(super) fn persist(self, connection: &Connection) -> Result<Self> {
        let user_data = PersistableUserData::from_connection_and_payload(
            connection,
            UserCreationState::FinalUserState(self),
        );
        user_data.persist()?;
        if let UserCreationState::FinalUserState(state) = user_data.into_payload() {
            Ok(state)
        } else {
            bail!("Unexpected user creation state")
        }
    }
}

pub(super) struct PersistableUserData<'a> {
    connection: &'a Connection,
    payload: UserCreationState,
}

impl PersistableUserData<'_> {
    pub(super) fn into_payload(self) -> UserCreationState {
        self.payload
    }

    pub(super) fn server_url(&self) -> &str {
        self.payload.server_url()
    }
}

impl<'a> Persistable<'a> for PersistableUserData<'a> {
    type Key = AsClientId;

    type SecondaryKey = AsClientId;

    type Payload = UserCreationState;

    const DATA_TYPE: DataType = DataType::ClientData;

    fn key(&self) -> &Self::Key {
        &self.payload.client_id()
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &self.payload.client_id()
    }

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
