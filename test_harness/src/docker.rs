// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    collections::{HashMap, HashSet},
    process::{Child, Command, Stdio},
    thread::sleep,
    time::Duration,
};

use once_cell::sync::Lazy;
use phnxapiclient::ApiClient;
use phnxtypes::{identifiers::Fqdn, DEFAULT_PORT_HTTP};

use crate::{test_scenarios::FederationTestScenario, TRACING};

pub(crate) struct DockerTestBed {
    // (server, db)
    servers: HashMap<Fqdn, (Child, Child)>,
    network_name: String,
}

impl Drop for DockerTestBed {
    fn drop(&mut self) {
        self.stop_all_servers();
        remove_network(&self.network_name);
    }
}

impl DockerTestBed {
    fn stop_all_servers(&mut self) {
        for (domain, _server) in self.servers.iter_mut() {
            tracing::info!("Stopping docker container of server {domain}");
            let server_container_name = format!("{}_server_container", domain);
            stop_docker_container(&server_container_name);
            let database_container_name = format!("{}_db_container", domain);
            stop_docker_container(&database_container_name);
        }
    }

    pub async fn new(scenario: &FederationTestScenario) -> Self {
        // Make sure that Docker is actually running
        assert_docker_is_running();

        let network_name = format!("{scenario}_network");
        // Create docker network
        create_network(&network_name);
        let servers = (0..scenario.number_of_servers())
            .into_iter()
            .map(|index| {
                let domain = format!("{}{}.com", scenario, index)
                    .try_into()
                    .expect("Invalid domain");
                tracing::info!("Starting server {domain}");
                let server = create_and_start_server_container(&domain, Some(&network_name));
                (domain.clone(), server)
            })
            .collect::<HashMap<_, _>>();

        Self {
            servers,
            network_name,
        }
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

        let mut env_variables = vec![test_scenario_env_variable, "TEST_LOG=true".to_owned()];

        for (index, server) in self.servers.keys().enumerate() {
            env_variables.push(format!("PHNX_SERVER_{}={}", index, server));
        }

        // Forward the random seed env variable
        if let Ok(seed) = std::env::var("PHNX_TEST_RANDOM_SEED") {
            env_variables.push(format!("PHNX_TEST_RANDOM_SEED={}", seed))
        };

        let test_runner_result = run_docker_container(
            &image_name,
            &container_name,
            &env_variables,
            // No hostname required for the test container
            None,
            Some(&self.network_name),
            None,
            &[],
            false,
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
    env_variables: &[String],
    hostname_option: Option<&str>,
    network_name_option: Option<&str>,
    port_option: Option<&str>,
    run_parameters: &[String],
    detach: bool,
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
    if let Some(port) = port_option {
        command.args(["-p", format!("{}", port).as_str()]);
    }
    command.args(["--rm"]);
    if detach {
        command.args(["-d"]);
    }
    command.args([image_name]);
    command.args(run_parameters);
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
) -> (Child, Child) {
    // First go into the workspace dir s.t. we can build the docker image.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    std::env::set_current_dir(manifest_dir.to_owned() + "/..").unwrap();

    let db_image_name = "postgres";
    let db_container_name = format!("{server_domain}_db_container");
    let db_domain = format!("db.{server_domain}");
    let db_user = "postgres";
    let db_password = "password";
    let db_name = "phnx_server_db";
    let db_port = "5432";

    let db_domain_env_variable = format!("PHNX_DB_DOMAIN={db_domain}");
    let db_user_env_variable = format!("POSTGRES_USER={db_user}");
    let db_password_env_variable = format!("POSTGRES_PASSWORD={db_password}");
    let db_name_env_variable = format!("POSTGRES_DB={db_name}");

    let db = run_docker_container(
        &db_image_name,
        &db_container_name,
        &[
            db_domain_env_variable,
            db_user_env_variable,
            db_password_env_variable,
            db_name_env_variable,
        ],
        Some(&db_domain),
        network_name_option,
        Some(db_port),
        &["-N".to_string(), "1000".to_string(), "-i".to_string()],
        false,
    );

    let server_image_name = "phnxserver_image";
    let server_container_name = format!("{server_domain}_server_container");

    build_docker_image("server/Dockerfile", &server_image_name);

    let server_domain_env_variable = format!("PHNX_APPLICATION_DOMAIN={}", server_domain);
    let server_db_user_env_variable = format!("PHNX_DATABASE_USERNAME={}", db_user);
    let server_db_password_env_variable = format!("PHNX_DATABASE_PASSWORD={}", db_password);
    let server_db_port_env_variable = format!("PHNX_DATABASE_PORT={}", db_port);
    let server_host_env_variable = format!("PHNX_DATABASE_HOST={}", db_domain);
    let server_db_name_env_variable = format!("PHNX_DATABASE_DATABASE_NAME={}", db_name);
    let server_sqlx_offline_env_variable = format!("SQLX_OFFLINE=true");
    let server = run_docker_container(
        &server_image_name,
        &server_container_name,
        &[
            server_domain_env_variable,
            server_host_env_variable,
            server_db_name_env_variable,
            server_db_user_env_variable,
            server_db_password_env_variable,
            server_db_port_env_variable,
            server_sqlx_offline_env_variable,
        ],
        Some(&server_domain.to_string()),
        network_name_option,
        None,
        &[],
        false,
    );

    (server, db)
}

/// This function has to be called from the container that runs the tests.
pub async fn wait_until_servers_are_up(domains: impl Into<HashSet<Fqdn>>) -> bool {
    let mut domains = domains.into();
    let clients: HashMap<Fqdn, ApiClient> = domains
        .iter()
        .map(|domain| {
            let domain_and_port = format!("http://{}:{}", domain, DEFAULT_PORT_HTTP);
            (
                domain.clone(),
                ApiClient::initialize(domain_and_port).unwrap(),
            )
        })
        .collect();

    // Do the health check
    let mut counter = 0;
    while !domains.is_empty() && counter < 10 {
        for (domain, client) in &clients {
            if client.health_check().await {
                domains.remove(&domain);
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
        counter += 1;
    }
    if counter == 10 {
        return false;
    } else {
        return true;
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
            != (format!(
                "Error response from daemon: network with name {} already exists\n",
                network_name
            ))
            .as_bytes()
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

pub async fn run_server_restart_test() {
    Lazy::force(&TRACING);

    // Make sure that Docker is actually running
    assert_docker_is_running();

    let server_domain = "example.com";
    let network_name = "server_restart_network";
    // Create docker network
    create_network(network_name);

    // Start server and db container
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    std::env::set_current_dir(manifest_dir.to_owned() + "/..").unwrap();

    let db_image_name = "postgres";
    let db_container_name = format!("{server_domain}_db_container");
    let db_domain = format!("db.{server_domain}");
    let db_user = "postgres";
    let db_password = "password";
    let db_name = "phnx_server_db";
    let db_port = "5432";

    let db_domain_env_variable = format!("PHNX_DB_DOMAIN={db_domain}");
    let db_user_env_variable = format!("POSTGRES_USER={db_user}");
    let db_password_env_variable = format!("POSTGRES_PASSWORD={db_password}");
    let db_name_env_variable = format!("POSTGRES_DB={db_name}");

    let _db = run_docker_container(
        &db_image_name,
        &db_container_name,
        &[
            db_domain_env_variable,
            db_user_env_variable,
            db_password_env_variable,
            db_name_env_variable,
        ],
        Some(&db_domain),
        Some(network_name),
        Some(db_port),
        &["-N".to_string(), "1000".to_string(), "-i".to_string()],
        false,
    );

    let server_image_name = "phnxserver_image";
    let server_container_name = format!("{server_domain}_server_container");

    build_docker_image("server/Dockerfile", &server_image_name);

    let server_domain_env_variable = format!("PHNX_APPLICATION_DOMAIN={}", server_domain);
    let server_db_user_env_variable = format!("PHNX_DATABASE_USERNAME={}", db_user);
    let server_db_password_env_variable = format!("PHNX_DATABASE_PASSWORD={}", db_password);
    let server_db_port_env_variable = format!("PHNX_DATABASE_PORT={}", db_port);
    let server_host_env_variable = format!("PHNX_DATABASE_HOST={}", db_domain);
    let server_db_name_env_variable = format!("PHNX_DATABASE_DATABASE_NAME={}", db_name);
    let server_sqlx_offline_env_variable = format!("SQLX_OFFLINE=true");

    tracing::info!("Starting phnx server");
    let _server = run_docker_container(
        &server_image_name,
        &server_container_name,
        &[
            server_domain_env_variable.clone(),
            server_host_env_variable.clone(),
            server_db_name_env_variable.clone(),
            server_db_user_env_variable.clone(),
            server_db_password_env_variable.clone(),
            server_db_port_env_variable.clone(),
            server_sqlx_offline_env_variable.clone(),
        ],
        Some(&server_domain.to_string()),
        Some(network_name),
        None,
        &[],
        false,
    );

    sleep(Duration::from_secs(3));

    tracing::info!("All servers are up, stopping server.");

    // Stop server container
    stop_docker_container(&server_container_name);

    sleep(Duration::from_secs(3));

    tracing::info!("Waited three seconds, starting server again.");

    // Start server container again
    let _server = run_docker_container(
        &server_image_name,
        &server_container_name,
        &[
            server_domain_env_variable,
            server_host_env_variable,
            server_db_name_env_variable,
            server_db_user_env_variable,
            server_db_password_env_variable,
            server_db_port_env_variable,
            server_sqlx_offline_env_variable,
        ],
        Some(&server_domain.to_string()),
        Some(network_name),
        None,
        &[],
        false,
    );

    sleep(Duration::from_secs(3));

    stop_docker_container(&server_container_name);
    stop_docker_container(&db_container_name);

    tracing::info!("Done running server restart test");
}
