// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxserver_test_harness::docker::run_server_restart_test;

#[actix_rt::test]
#[ignore]
#[tracing::instrument(name = "Server restart test", skip_all)]
async fn skip_db_creation() {
    run_server_restart_test().await
}
