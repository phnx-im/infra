// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

impl<T: Notifiable> SelfUser<T> {
    pub(super) fn from_user_data(
        sqlite_connection: Connection,
        user_data: UserData,
        notification_hub_option: Option<NotificationHub<T>>,
    ) -> Result<Self> {
        let UserData {
            as_client_id,
            qs_client_id,
            key_store,
            _qs_user_id,
            server_url,
        } = user_data;

        let api_clients = ApiClients::new(as_client_id.user_name().domain(), server_url.clone());

        let user = Self {
            sqlite_connection,
            crypto_backend: PhnxOpenMlsProvider::new(as_client_id),
            api_clients,
            key_store,
            _qs_user_id: QsUserId::random(),
            qs_client_id,
            notification_hub_option: Mutex::new(notification_hub_option),
        };
        Ok(user)
    }

    pub fn load(as_client_id: AsClientId, notification_hub: NotificationHub<T>) -> Result<Self> {
        log::debug!("Loading client {}", as_client_id);

        let db_path = db_path(&as_client_id);
        let sqlite_connection = Connection::open(db_path)?;

        let user_data =
            PersistableUserData::load_one(&sqlite_connection, Some(&as_client_id), None)?
                .ok_or(anyhow!("Can't find data for this client id."))?
                .into_user_data();

        Self::from_user_data(sqlite_connection, user_data, Some(notification_hub))
    }
}

#[derive(Serialize, Deserialize)]
pub(super) struct UserData {
    pub(super) as_client_id: AsClientId,
    pub(super) qs_client_id: QsClientId,
    pub(super) key_store: MemoryUserKeyStore,
    pub(super) _qs_user_id: QsUserId,
    pub(super) server_url: String,
}

impl UserData {
    pub(super) fn persist(self, connection: &Connection) -> Result<Self, PersistenceError> {
        let p_user_data = PersistableUserData::from_connection_and_payload(connection, self);
        p_user_data.persist()?;
        Ok(p_user_data.into_user_data())
    }
}

struct PersistableUserData<'a> {
    connection: &'a Connection,
    payload: UserData,
}

impl PersistableUserData<'_> {
    fn into_user_data(self) -> UserData {
        self.payload
    }
}

impl<'a> Persistable<'a> for PersistableUserData<'a> {
    type Key = AsClientId;

    type SecondaryKey = AsClientId;

    type Payload = UserData;

    const DATA_TYPE: DataType = DataType::ClientData;

    fn key(&self) -> &Self::Key {
        &self.payload.as_client_id
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &self.payload.as_client_id
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
