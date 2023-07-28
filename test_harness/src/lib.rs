// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use once_cell::sync::Lazy;

use crate::docker::{
    create_and_start_server_container, create_and_start_test_container, create_network,
};
use phnxserver::telemetry::{get_subscriber, init_subscriber};

pub mod docker;
pub mod test_scenarios;
pub mod utils;

pub(crate) const FEDERATION_TEST_OWNER_DOMAIN: &str = "phnxowningserver.com";
pub(crate) const FEDERATION_TEST_GUEST_DOMAIN: &str = "phnxguestserver.com";

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

pub async fn run_federation_scenario() {
    tracing::info!("Running federation test scenario");
    Lazy::force(&TRACING);
    let network_name = "federation_test_network";
    create_network(network_name).await;
    // This spawns a child process that runs the server container.
    // Alternatively, we could use docker detach.
    let _owning_child =
        create_and_start_server_container(FEDERATION_TEST_OWNER_DOMAIN.into(), Some(network_name))
            .await;
    let _guest_child =
        create_and_start_server_container(FEDERATION_TEST_GUEST_DOMAIN.into(), Some(network_name))
            .await;

    create_and_start_test_container("federation", Some(network_name)).await;
}
