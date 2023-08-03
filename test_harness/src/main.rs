// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use once_cell::sync::Lazy;
use phnxserver::telemetry::{get_subscriber, init_subscriber};
use phnxserver_test_harness::test_scenarios::{
    connect_federated_users::connect_federated_users_runner,
    federated_group_operations::federated_group_operations_runner, FederationTestScenario,
};

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();
    // This allows us to choose not to capture traces for tests that pass.
    // To get all logs just run `TEST_LOG=true cargo test health_check_works | bunyan`.
    // bunyan can be installed via `cargo install bunyan`.
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    }
});

#[tokio::main]
async fn main() {
    Lazy::force(&TRACING);
    let scenario_name = std::env::var("PHNX_TEST_SCENARIO").unwrap().into();

    match scenario_name {
        FederationTestScenario::ConnectUsers => connect_federated_users_runner().await,
        FederationTestScenario::GroupOperations => federated_group_operations_runner().await,
    }
}
