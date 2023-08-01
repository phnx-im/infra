// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxserver_test_harness::test_scenarios::federation::connect_federated_users_scenario;

#[actix_rt::test]
#[tracing::instrument(name = "Connect federated users test", skip_all)]
async fn connect_federated_users() {
    connect_federated_users_scenario().await;
}
