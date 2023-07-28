// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxserver_test_harness::run_federation_scenario;

#[actix_rt::test]
#[ignore]
#[tracing::instrument(name = "Connect federated users test", skip_all)]
async fn connect_federated_users() {
    run_federation_scenario().await;
}
