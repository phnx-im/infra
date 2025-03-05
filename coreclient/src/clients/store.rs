// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::bail;
use phnxtypes::messages::push_token::PushToken;

use super::{
    create_user::{
        AsRegisteredUserState, BasicUserData, PersistedUserState, PostRegistrationInitState,
        QsRegisteredUserState, UnfinalizedRegistrationState,
    },
    *,
};

/// WARNING: This enum is stored in sqlite as a blob. If any changes are made to
/// this enum, a new version in `StorableUserCreationState` must be created.
#[derive(Serialize, Deserialize)]
pub(crate) enum UserCreationState {
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

    pub(super) async fn new(
        client_db: &SqlitePool,
        phnx_db: &SqlitePool,
        as_client_id: AsClientId,
        server_url: impl ToString,
        password: &str,
        push_token: Option<PushToken>,
    ) -> Result<Self> {
        let client_record = ClientRecord::new(as_client_id.clone());
        client_record.store(phnx_db).await?;

        let basic_user_data = BasicUserData {
            as_client_id: as_client_id.clone(),
            server_url: server_url.to_string(),
            password: password.to_string(),
            push_token,
        };

        // Create user profile entry for own user.
        UserProfile::new(as_client_id.user_name(), None, None)
            .upsert(client_db, &mut StoreNotifier::noop())
            .await?;

        let user_creation_state = UserCreationState::BasicUserData(basic_user_data);

        user_creation_state.store(client_db).await?;

        Ok(user_creation_state)
    }

    pub(super) async fn step(
        self,
        phnx_db: &SqlitePool,
        client_db: &SqlitePool,
        api_clients: &ApiClients,
    ) -> Result<Self> {
        // If we're already in the final state, there is nothing to do.
        if matches!(self, UserCreationState::FinalUserState(_)) {
            return Ok(self);
        }

        let new_state = match self {
            UserCreationState::BasicUserData(state) => {
                let mut connection = client_db.acquire().await?;
                let state = state
                    .prepare_as_registration(&mut connection, api_clients)
                    .await?;
                Self::InitialUserState(state)
            }
            UserCreationState::InitialUserState(state) => {
                Self::PostRegistrationInitState(state.initiate_as_registration(api_clients).await?)
            }
            UserCreationState::PostRegistrationInitState(state) => {
                let mut connection = client_db.acquire().await?;
                let state = state.process_server_response(&mut connection).await?;
                Self::UnfinalizedRegistrationState(state)
            }
            UserCreationState::UnfinalizedRegistrationState(state) => {
                Self::AsRegisteredUserState(state.finalize_as_registration(api_clients).await?)
            }
            UserCreationState::AsRegisteredUserState(state) => {
                Self::QsRegisteredUserState(state.register_with_qs(api_clients).await?)
            }
            UserCreationState::QsRegisteredUserState(state) => {
                let mut connection = client_db.acquire().await?;
                let persisted_user_state = state
                    .upload_key_packages(&mut connection, api_clients)
                    .await?;
                Self::FinalUserState(persisted_user_state)
            }
            UserCreationState::FinalUserState(_) => self,
        };

        new_state.store(client_db).await?;

        // If we just transitioned into the final state, we need to update the
        // client record.
        if let UserCreationState::FinalUserState(_) = new_state {
            let mut client_record = ClientRecord::load(phnx_db, new_state.client_id())
                .await?
                .ok_or(anyhow!("Client record not found"))?;
            client_record.finish();
            client_record.store(phnx_db).await?;
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
        phnx_db: &SqlitePool,
        client_db: &SqlitePool,
        api_clients: &ApiClients,
    ) -> Result<PersistedUserState> {
        while !matches!(self, UserCreationState::FinalUserState(_)) {
            self = self.step(phnx_db, client_db, api_clients).await?
        }

        self.final_state()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientRecordState {
    InProgress,
    Finished,
}

impl ClientRecordState {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            ClientRecordState::InProgress => "in_progress",
            ClientRecordState::Finished => "finished",
        }
    }

    pub(crate) fn from_str(state: &str) -> Option<Self> {
        match state {
            "in_progress" => Some(ClientRecordState::InProgress),
            "finished" => Some(ClientRecordState::Finished),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientRecord {
    pub as_client_id: AsClientId,
    pub client_record_state: ClientRecordState,
    pub created_at: DateTime<Utc>,
    pub is_default: bool,
}

impl ClientRecord {
    pub(super) fn new(as_client_id: AsClientId) -> Self {
        Self {
            as_client_id,
            client_record_state: ClientRecordState::InProgress,
            created_at: Utc::now(),
            is_default: false,
        }
    }

    pub(super) fn finish(&mut self) {
        self.client_record_state = ClientRecordState::Finished;
    }
}
