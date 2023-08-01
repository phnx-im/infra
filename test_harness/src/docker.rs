// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    collections::{HashMap, HashSet},
    process::{Child, Command, Stdio},
};

use phnxapiclient::{ApiClient, DomainOrAddress, TransportEncryption};
use phnxbackend::qs::Fqdn;

const DOCKER_NETWORK_NAME: &str = "phnx_test_network";

pub(crate) struct DockerTestBed {
    servers: HashMap<Fqdn, Child>,
}

impl Drop for DockerTestBed {
    fn drop(&mut self) {
        self.stop_all_servers();
        remove_network(DOCKER_NETWORK_NAME);
    }
}

impl DockerTestBed {
    fn stop_all_servers(&mut self) {
        for (domain, _server) in self.servers.iter_mut() {
            tracing::info!("Stopping docker container of server {domain}");
            let server_container_name = format!("{}_server_container", domain);
            stop_docker_container(&server_container_name);
        }
    }
    pub async fn new(domains: &[&str]) -> Self {
        // Make sure that Docker is actually running
        assert_docker_is_running();

        // Create docker network
        create_network(DOCKER_NETWORK_NAME);

        let domains: HashSet<Fqdn> = domains.iter().map(|&domain| domain.into()).collect();

        let mut servers = HashMap::new();
        for domain in domains.iter() {
            let server = create_and_start_server_container(&domain, Some(DOCKER_NETWORK_NAME));
            servers.insert(domain.clone(), server);
        }

        Self { servers }
    }

    pub fn start_test(&mut self, test_scenario_name: &str) {
        // This function builds the test image and starts the container.

        // First go into the workspace dir s.t. we can build the docker image.
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        std::env::set_current_dir(manifest_dir.to_owned() + "/..").unwrap();

        let image_name = format!("{}_image", test_scenario_name);
        let container_name = format!("{}_container", test_scenario_name);

        build_docker_image("test_harness/Dockerfile", &image_name);

        let test_scenario_env_variable = format!("PHNX_TEST_SCENARIO={}", test_scenario_name);

        tracing::info!("Running docker image");
        let test_runner_result = run_docker_container(
            &image_name,
            &container_name,
            &[&test_scenario_env_variable, "TEST_LOG=true"],
            // No hostname required for the test container
            None,
            Some(DOCKER_NETWORK_NAME),
        )
        .wait()
        .unwrap();

        assert!(test_runner_result.success());
    }
}

fn build_docker_image(path_to_docker_file: &str, image_name: &str) {
    tracing::info!("Building docker image: {}", image_name);
    let build_output = Command::new("docker")
        .arg("build")
        .arg("-t")
        .arg(image_name)
        .arg("-f")
        .arg(path_to_docker_file)
        .arg(".")
        .status()
        .expect("failed to execute process");

    debug_assert!(build_output.success());
}

fn run_docker_container(
    image_name: &str,
    container_name: &str,
    env_variables: &[&str],
    hostname_option: Option<&str>,
    network_name_option: Option<&str>,
) -> Child {
    let mut command = Command::new("docker");
    command.arg("run");
    for env_variable in env_variables {
        command.args(["--env", env_variable]);
    }
    if let Some(network_name) = network_name_option {
        command.args(["--network", network_name]);
    }
    if let Some(hostname) = hostname_option {
        command.args(["--hostname", hostname]);
    }
    command.args(["--name", container_name]);
    command.args(["--rm", image_name]);
    command.spawn().unwrap()
}

fn stop_docker_container(container_name: &str) {
    let status = Command::new("docker")
        .args(["stop", container_name])
        .status()
        .unwrap();
    assert!(status.success());
}

fn create_and_start_server_container(
    server_domain: &Fqdn,
    network_name_option: Option<&str>,
) -> Child {
    // First go into the workspace dir s.t. we can build the docker image.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    std::env::set_current_dir(manifest_dir.to_owned() + "/..").unwrap();

    let image_name = "phnxserver_image";
    let container_name = format!("{server_domain}_server_container");

    build_docker_image("server/Dockerfile", &image_name);

    let server_domain_env_variable = format!("PHNX_SERVER_DOMAIN={}", server_domain);
    run_docker_container(
        &image_name,
        &container_name,
        &[&server_domain_env_variable],
        Some(&server_domain.to_string()),
        network_name_option,
    )
}

/// This function has to be called from the container that runs the tests.
pub(crate) async fn wait_until_servers_are_up(domains: impl Into<HashSet<Fqdn>>) {
    let mut domains = domains.into();
    let clients: Vec<ApiClient> = domains
        .iter()
        .map(|domain| ApiClient::initialize(domain.clone(), TransportEncryption::Off).unwrap())
        .collect::<Vec<ApiClient>>();

    // Do the health check
    while !domains.is_empty() {
        for client in &clients {
            if client.health_check().await {
                if let DomainOrAddress::Domain(domain) = client.domain_or_address() {
                    domains.remove(domain);
                } else {
                    panic!("Expected domain")
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(2))
    }
}

fn create_network(network_name: &str) {
    tracing::info!("Creating network: {}", network_name);
    let command_output = Command::new("docker")
        .arg("network")
        .arg("create")
        .arg(network_name)
        .output()
        .expect("failed to execute process");

    if !command_output.status.success()
        && command_output.stderr
            != b"Error response from daemon: network with name phnx_test_network already exists\n"
    {
        panic!("Failed to create network: {:?}", command_output);
    }
}

fn remove_network(network_name: &str) {
    tracing::info!("Remove network: {}", network_name);
    let command_output = Command::new("docker")
        .arg("network")
        .arg("rm")
        .arg(network_name)
        .status()
        .expect("failed to execute process");

    assert!(command_output.success());
}

fn assert_docker_is_running() {
    if !Command::new("docker")
        .arg("version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap()
        .success()
    {
        panic!("Docker is not running. Please start docker and try again.");
    }
}
