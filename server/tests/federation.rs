// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxserver_test_harness::test_scenarios::{run_test_scenario, FederationTestScenario};

#[actix_rt::test]
#[tracing::instrument(name = "Connect federated users test", skip_all)]
async fn connect_federated_users() {
    run_test_scenario(FederationTestScenario::ConnectUsers).await;
}

#[actix_rt::test]
#[tracing::instrument(name = "Federated group operations test", skip_all)]
async fn federated_group_operations() {
    run_test_scenario(FederationTestScenario::GroupOperations).await;
}
