// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{collections::HashSet, process::ExitCode};

use once_cell::sync::Lazy;
use phnxserver::telemetry::{get_subscriber, init_subscriber};
use phnxserver_test_harness::{
    docker::wait_until_servers_are_up,
    test_scenarios::{
        basic_group_operations::{
            connect_users_runner, invite_to_group_runner, leave_group_runner,
            remove_from_group_runner,
        },
        federated_group_operations::group_operations_runner,
        randomized_operations::randomized_operations_runner,
        FederationTestScenario,
    },
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
async fn main() -> ExitCode {
    Lazy::force(&TRACING);
    let scenario_name = std::env::var("PHNX_TEST_SCENARIO").unwrap().into();

    let mut counter = 0;
    let mut domains = HashSet::new();
    while let Ok(domain_name) = std::env::var(format!("PHNX_SERVER_{}", counter)) {
        domains.insert(domain_name.into());
        counter += 1;
    }
    if !wait_until_servers_are_up(domains.clone()).await {
        return ExitCode::FAILURE;
    };
    let domains_vec = domains.into_iter().collect::<Vec<_>>();

    match scenario_name {
        FederationTestScenario::ConnectUsers => connect_users_runner(&domains_vec).await,
        FederationTestScenario::GroupOperations => group_operations_runner(&domains_vec).await,
        FederationTestScenario::InviteToGroup => invite_to_group_runner(&domains_vec).await,
        FederationTestScenario::RemoveFromGroup => remove_from_group_runner(&domains_vec).await,
        FederationTestScenario::LeaveGroup => leave_group_runner(&domains_vec).await,
        FederationTestScenario::RandomizedOperations => {
            randomized_operations_runner(&domains_vec).await
        }
    };
    return ExitCode::SUCCESS;
}
