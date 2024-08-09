// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::api_clients::ApiClients;
use crate::{
    clients::store::{ClientRecord, ClientRecordState, UserCreationState},
    utils::{
        persistence::{SqliteConnection, Storable},
        set_up_database,
    },
};
use phnxserver_test_harness::utils::setup::TestBackend;
use phnxtypes::identifiers::{AsClientId, SafeTryInto};
use rusqlite::Connection;

#[actix_rt::test]
async fn user_stages() {
    // Set up backend
    let setup = TestBackend::single().await;

    let user_name = "alice@example.com";
    let as_client_id = AsClientId::random(SafeTryInto::try_into(user_name).unwrap()).unwrap();

    let phnx_db_connection = Connection::open_in_memory().unwrap();
    let mut client_db_connection = Connection::open_in_memory().unwrap();

    // Set up the client db
    set_up_database(&mut client_db_connection).unwrap();
    // Set up phnx db
    ClientRecord::create_table(&phnx_db_connection).unwrap();

    let server_url = setup.url().unwrap();
    let api_clients = ApiClients::new(as_client_id.user_name().domain(), server_url.clone());

    let computed_state = UserCreationState::new(
        &client_db_connection,
        &phnx_db_connection,
        as_client_id.clone(),
        server_url,
        user_name,
        None,
    )
    .unwrap();

    // There should now be a client record state in the phnx db.
    let client_records = ClientRecord::load_all(&phnx_db_connection).unwrap();
    assert!(client_records.len() == 1);
    let client_record = client_records.first().unwrap();
    assert!(client_record.as_client_id == as_client_id);
    assert!(matches!(
        client_record.client_record_state,
        ClientRecordState::InProgress
    ));

    // If we load a user state now, it should be the basic user data state.
    let loaded_state = UserCreationState::load(&client_db_connection, &as_client_id)
        .unwrap()
        .unwrap();
    assert!(matches!(loaded_state, UserCreationState::BasicUserData(_)));
    assert_eq!(
        phnxtypes::codec::to_vec(&computed_state).unwrap(),
        phnxtypes::codec::to_vec(&loaded_state).unwrap()
    );

    let client_db_connection_mutex = SqliteConnection::new(client_db_connection);
    let phnx_db_connection_mutex = SqliteConnection::new(phnx_db_connection);
    // We now continue down the path of creating a user.
    let computed_state = loaded_state
        .step(
            phnx_db_connection_mutex.clone(),
            client_db_connection_mutex.clone(),
            &api_clients,
        )
        .await
        .unwrap();

    // If we load a user state now, it should be the initial user state.
    let client_db_connection = client_db_connection_mutex.lock().await;
    let loaded_state = UserCreationState::load(&client_db_connection, &as_client_id)
        .unwrap()
        .unwrap();
    assert!(matches!(
        loaded_state,
        UserCreationState::InitialUserState(_)
    ));
    assert_eq!(
        phnxtypes::codec::to_vec(&computed_state).unwrap(),
        phnxtypes::codec::to_vec(&loaded_state).unwrap()
    );
    drop(client_db_connection);

    // We take the next step
    let computed_state = loaded_state
        .step(
            phnx_db_connection_mutex.clone(),
            client_db_connection_mutex.clone(),
            &api_clients,
        )
        .await
        .unwrap();

    let client_db_connection = client_db_connection_mutex.lock().await;
    // If we load a user state now, it should be the post registration init state.
    let loaded_state = UserCreationState::load(&client_db_connection, &as_client_id)
        .unwrap()
        .unwrap();
    assert!(matches!(
        loaded_state,
        UserCreationState::PostRegistrationInitState(_)
    ));
    assert_eq!(
        phnxtypes::codec::to_vec(&computed_state).unwrap(),
        phnxtypes::codec::to_vec(&loaded_state).unwrap()
    );
    drop(client_db_connection);

    // We take the next step
    let computed_state = loaded_state
        .step(
            phnx_db_connection_mutex.clone(),
            client_db_connection_mutex.clone(),
            &api_clients,
        )
        .await
        .unwrap();

    // If we load a user state now, it should be the unfinalized registration state.
    let client_db_connection = client_db_connection_mutex.lock().await;
    let loaded_state = UserCreationState::load(&client_db_connection, &as_client_id)
        .unwrap()
        .unwrap();
    assert!(matches!(
        loaded_state,
        UserCreationState::UnfinalizedRegistrationState(_)
    ));
    assert_eq!(
        phnxtypes::codec::to_vec(&computed_state).unwrap(),
        phnxtypes::codec::to_vec(&loaded_state).unwrap()
    );
    drop(client_db_connection);

    // We take the next step
    let computed_state = loaded_state
        .step(
            phnx_db_connection_mutex.clone(),
            client_db_connection_mutex.clone(),
            &api_clients,
        )
        .await
        .unwrap();

    // If we load a user state now, it should be the AS registered user state.
    let client_db_connection = client_db_connection_mutex.lock().await;
    let loaded_state = UserCreationState::load(&client_db_connection, &as_client_id)
        .unwrap()
        .unwrap();
    assert!(matches!(
        loaded_state,
        UserCreationState::AsRegisteredUserState(_)
    ));
    assert_eq!(
        phnxtypes::codec::to_vec(&computed_state).unwrap(),
        phnxtypes::codec::to_vec(&loaded_state).unwrap()
    );
    drop(client_db_connection);

    // We take the next step
    let computed_state = loaded_state
        .step(
            phnx_db_connection_mutex.clone(),
            client_db_connection_mutex.clone(),
            &api_clients,
        )
        .await
        .unwrap();

    // If we load a user state now, it should be the QS registered user state.
    let client_db_connection = client_db_connection_mutex.lock().await;
    let loaded_state = UserCreationState::load(&client_db_connection, &as_client_id)
        .unwrap()
        .unwrap();
    assert!(matches!(
        loaded_state,
        UserCreationState::QsRegisteredUserState(_)
    ));
    assert_eq!(
        phnxtypes::codec::to_vec(&computed_state).unwrap(),
        phnxtypes::codec::to_vec(&loaded_state).unwrap()
    );
    drop(client_db_connection);

    // We take the final step
    let computed_state = loaded_state
        .step(
            phnx_db_connection_mutex.clone(),
            client_db_connection_mutex.clone(),
            &api_clients,
        )
        .await
        .unwrap();

    // If we load a user state now, it should be the final user state.
    let client_db_connection = client_db_connection_mutex.lock().await;
    let loaded_state = UserCreationState::load(&client_db_connection, &as_client_id)
        .unwrap()
        .unwrap();
    assert!(matches!(loaded_state, UserCreationState::FinalUserState(_)));
    assert_eq!(
        phnxtypes::codec::to_vec(&computed_state).unwrap(),
        phnxtypes::codec::to_vec(&loaded_state).unwrap()
    );
}
