// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{docker::DockerTestBed, init_test_tracing};

pub mod basic_group_operations;
pub mod federated_group_operations;
pub mod randomized_operations;

// When adding a test scenario, don't forget to add it to the `From<String>`
// implementation.
#[derive(Debug, Clone)]
pub enum FederationTestScenario {
    ConnectUsers,
    InviteToGroup,
    RemoveFromGroup,
    LeaveGroup,
    GroupOperations,
    RandomizedOperations,
}

impl FederationTestScenario {
    pub(crate) fn number_of_servers(&self) -> usize {
        match self {
            Self::ConnectUsers => basic_group_operations::NUMBER_OF_SERVERS,
            Self::InviteToGroup => basic_group_operations::NUMBER_OF_SERVERS,
            Self::GroupOperations => federated_group_operations::NUMBER_OF_SERVERS,
            Self::RemoveFromGroup => basic_group_operations::NUMBER_OF_SERVERS,
            Self::LeaveGroup => basic_group_operations::NUMBER_OF_SERVERS,
            Self::RandomizedOperations => randomized_operations::NUMBER_OF_SERVERS,
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
            "removefromgroup" => Self::RemoveFromGroup,
            "leavegroup" => Self::LeaveGroup,
            "invitetogroup" => Self::InviteToGroup,
            "randomizedoperations" => Self::RandomizedOperations,
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
    init_test_tracing();
    tracing::info!("Running federation test scenario: {}", scenario);

    let mut docker = DockerTestBed::new(&scenario).await;

    docker.start_test(&scenario.clone().to_string());

    tracing::info!("Done running federation test scenario: {}", scenario);
}
