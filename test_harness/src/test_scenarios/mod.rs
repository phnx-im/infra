// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use once_cell::sync::Lazy;

use crate::{docker::DockerTestBed, TRACING};

pub mod connect_federated_users;
pub mod federated_group_operations;

#[derive(Debug, Clone)]
pub enum FederationTestScenario {
    ConnectUsers,
    GroupOperations,
}

impl FederationTestScenario {
    pub(crate) fn number_of_servers(&self) -> usize {
        match self {
            Self::ConnectUsers => connect_federated_users::NUMBER_OF_SERVERS,
            Self::GroupOperations => federated_group_operations::NUMBER_OF_SERVERS,
        }
    }
}

impl From<String> for FederationTestScenario {
    fn from(value: String) -> Self {
        // Scenario name will later be used to generate domain names and should
        // thus be lowercase and generally valid as a domain name (without dots)
        match value.as_str() {
            "connectusers" => Self::ConnectUsers,
            "groupoperations" => Self::GroupOperations,
            other => panic!("Unknown federation test scenario: {}", other),
        }
    }
}

impl std::fmt::Display for FederationTestScenario {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let string = format!("{:?}", self).to_lowercase();
        write!(f, "{}", string)
    }
}

pub async fn run_test_scenario(scenario: FederationTestScenario) {
    Lazy::force(&TRACING);
    let scenario_string = scenario.to_string();
    tracing::info!("Running federation test scenario: {}", scenario_string);

    let mut docker = DockerTestBed::new(scenario).await;

    docker.start_test(&scenario_string)
}
