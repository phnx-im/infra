// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Tests for DS endpoints.

use phnxbackend::crypto::kdf::keys::InitialClientKdfKey;
use uuid::Uuid;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

use crate::{ds_api::DsRequestError, ApiClient};

// Test the DS endpoint for group creation.
//#[tokio::test]
// TODO: Fix this test when the DS is ready.
//#[should_panic]
async fn test_ds_create_group() {
    // Happy path
    // We expect to receive a group ID in return.

    /* let mock_server = MockServer::start().await;

    let group_id = GroupId(Uuid::new_v4());

    let dummy_create_group_params = CreateGroupParams {
        initial_secret: InitialClientKdfKey::dummy_value(),
        index: LeafNodeRef(Vec::new()),
        queue_config: QueueConfig::dummy_config(),
    };

    Mock::given(method("POST"))
        .and(path(ENDPOINT_DS_CREATE_GROUP))
        .respond_with(ResponseTemplate::new(200).set_body_json(&group_id))
        .mount(&mock_server)
        .await;

    let client = ApiClient::initialize(mock_server.uri(), crate::TransportEncryption::Off)
        .expect("Failed to initialize client");
    let res = client.ds_create_group(dummy_create_group_params).await;

    assert!(res.is_ok());
    assert_eq!(res.unwrap(), group_id);

    // Bad request
    // We expect to receive a bad request error.

    let dummy_create_group_params = CreateGroupParams {
        initial_secret: InitialClientKdfKey::dummy_value(),
        index: LeafNodeRef(Vec::new()),
        queue_config: QueueConfig::dummy_config(),
    };

    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(ENDPOINT_DS_CREATE_GROUP))
        .respond_with(ResponseTemplate::new(400))
        .mount(&mock_server)
        .await;

    let client = ApiClient::initialize(mock_server.uri(), crate::TransportEncryption::Off)
        .expect("Failed to initialize client");
    let res = client.ds_create_group(dummy_create_group_params).await;

    assert!(res.is_err());
    matches!(res.unwrap_err(), DsCreateGroupError::BadRequest); */
}
