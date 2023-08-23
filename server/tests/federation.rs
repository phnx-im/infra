// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxserver_test_harness::test_scenarios::{run_test_scenario, FederationTestScenario};

#[actix_rt::test]
#[ignore]
#[tracing::instrument(name = "Connect federated users test", skip_all)]
async fn connect_federated_users() {
    run_test_scenario(FederationTestScenario::ConnectUsers).await;
}

#[actix_rt::test]
#[ignore]
#[tracing::instrument(name = "Federated group operations test", skip_all)]
async fn federated_group_operations() {
    run_test_scenario(FederationTestScenario::GroupOperations).await;
}

#[actix_rt::test]
#[ignore]
#[tracing::instrument(name = "Federated group invitations test", skip_all)]
async fn invite_federated_users() {
    run_test_scenario(FederationTestScenario::InviteToGroup).await;
}

#[actix_rt::test]
#[ignore]
#[tracing::instrument(name = "Federated group removal test", skip_all)]
async fn remove_federated_users() {
    run_test_scenario(FederationTestScenario::RemoveFromGroup).await;
}

#[actix_rt::test]
#[ignore]
#[tracing::instrument(name = "Leave federated group test", skip_all)]
async fn leave_federated_group() {
    run_test_scenario(FederationTestScenario::LeaveGroup).await;
}

#[actix_rt::test]
#[ignore]
#[tracing::instrument(name = "Randomized operations test", skip_all)]
async fn randomized_federated_operations() {
    run_test_scenario(FederationTestScenario::RandomizedOperations).await;
}
