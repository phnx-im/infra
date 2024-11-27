// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use core::panic;
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

use container::Container;

mod container;

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

        let mut test_runner_builder = Container::builder(&image_name, &container_name)
            .with_env(&test_scenario_env_variable)
            .with_env("TEST_LOG=true")
            .with_network(&self.network_name)
            .with_detach(false);

        for (index, server) in self.servers.keys().enumerate() {
            test_runner_builder =
                test_runner_builder.with_env(&format!("PHNX_SERVER_{}={}", index, server));
        }

        // Forward the random seed env variable
        if let Ok(seed) = std::env::var("PHNX_TEST_RANDOM_SEED") {
            test_runner_builder =
                test_runner_builder.with_env(&format!("PHNX_TEST_RANDOM_SEED={}", seed));
        };

        let test_runner_result = test_runner_builder.build().run().wait().unwrap();

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

    // Set the env variable in which to generate the TLS certs
    let cert_dir = "backend/test_certs";
    let absolute_cert_dir = std::env::current_dir().unwrap().join(cert_dir);
    std::env::set_var("TEST_CERT_DIR_NAME", cert_dir);
    // Call script to generate the TLS certs
    let cert_gen_output = Command::new("bash")
        .arg("backend/scripts/generate_test_certs.sh")
        .output()
        .expect("failed to execute process");

    println!("Output of cert generation: {:?}", cert_gen_output);

    assert!(cert_gen_output.status.success());

    sleep(Duration::from_secs(5));

    let ls_output = Command::new("ls")
        .args(["-lash", absolute_cert_dir.to_str().unwrap()])
        .output()
        .expect("failed to execute process");

    println!("Output of ls: {:?}", ls_output);

    assert!(ls_output.status.success());

    // Chown the certs to the postgres user
    //let chown_output = Command::new("chown")
    //    .args(["-R", "70", absolute_cert_dir.to_str().unwrap()])
    //    .output()
    //    .expect("failed to execute process");

    //println!("Output of chown: {:?}", chown_output);

    //assert!(chown_output.status.success());

    let mut db_container = Container::builder(db_image_name, &db_container_name)
        .with_port(db_port)
        .with_hostname(&db_domain)
        .with_env(&db_domain_env_variable)
        .with_env(&db_user_env_variable)
        .with_env(&db_password_env_variable)
        .with_env(&db_name_env_variable)
        .with_volume(&format!(
            "{}:/etc/postgres_certs:rw",
            absolute_cert_dir.to_str().unwrap()
        ))
        .with_run_parameters(&["-N", "1000"])
        .with_run_parameters(&["-c", "ssl=on"])
        .with_run_parameters(&["-c", "ssl_cert_file=/etc/postgres_certs/server.crt"])
        .with_run_parameters(&["-c", "ssl_key_file=/etc/postgres_certs/server.key"])
        .with_run_parameters(&["-c", "ssl_ca_file=/etc/postgres_certs/root.crt"])
        .with_detach(false);

    if let Some(network_name) = network_name_option {
        db_container = db_container.with_network(network_name);
    }

    let db = db_container.build().run();

    let server_image_name = "phnxserver_image";

    build_docker_image("server/Dockerfile", server_image_name);

    // Chown the certs to the postgres user (we do this after building the
    // server image to give the postgres container time to start)
    //docker_exec(
    //    &db_container_name,
    //    "root",
    //    &[
    //        "chown",
    //        "-R",
    //        "postgres:postgres",
    //        "/etc/postgres_certs/server.crt",
    //    ],
    //);
    //docker_exec(
    //    &db_container_name,
    //    "root",
    //    &[
    //        "chown",
    //        "-R",
    //        "postgres:postgres",
    //        "/etc/postgres_certs/server.key",
    //    ],
    //);
    //docker_exec(
    //    &db_container_name,
    //    "root",
    //    &[
    //        "chown",
    //        "-R",
    //        "postgres:postgres",
    //        "/etc/postgres_certs/root.crt",
    //    ],
    //);
    //docker_exec(
    //    &db_container_name,
    //    "root",
    //    &["chmod", "600", "/etc/postgres_certs/server.key"],
    //);

    let mut server_container = Container::builder(
        server_image_name,
        &format!("{server_domain}_server_container"),
    )
    .with_env(&format!("PHNX_APPLICATION_DOMAIN={server_domain}"))
    .with_env(&format!("PHNX_DATABASE_USERNAME={db_user}"))
    .with_env(&format!("PHNX_DATABASE_PASSWORD={db_password}"))
    .with_env(&format!("PHNX_DATABASE_PORT={db_port}"))
    .with_env(&format!("PHNX_DATABASE_HOST={db_domain}"))
    .with_env(&format!("PHNX_DATABASE_NAME={db_name}"))
    .with_env("PHNX_DATABASE_CACERTPATH=/test_certs/root.crt")
    .with_env("SQLX_OFFLINE=true")
    .with_hostname(&server_domain.to_string())
    .with_volume(&format!(
        "{}:/test_certs:ro",
        absolute_cert_dir.to_str().unwrap()
    ))
    .with_detach(false);

    if let Some(network_name) = network_name_option {
        server_container = server_container.with_network(network_name);
    }

    let server = server_container.build().run();

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
                domains.remove(domain);
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
        counter += 1;
    }
    counter != 10
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

    let db_builder = Container::builder("postgres", &db_container_name)
        .with_port(&db_port)
        .with_hostname(&db_domain)
        .with_network(network_name)
        .with_env(&db_domain_env_variable)
        .with_env(&db_user_env_variable)
        .with_env(&db_password_env_variable)
        .with_env(&db_name_env_variable)
        .with_run_parameters(&["-N", "1000"])
        .with_detach(false);

    let _db = db_builder.build().run();

    let server_image_name = "phnxserver_image";
    let server_container_name = format!("{server_domain}_server_container");

    build_docker_image("server/Dockerfile", server_image_name);

    let server_domain_env_variable = format!("PHNX_APPLICATION_DOMAIN={}", server_domain);
    let server_db_user_env_variable = format!("PHNX_DATABASE_USERNAME={}", db_user);
    let server_db_password_env_variable = format!("PHNX_DATABASE_PASSWORD={}", db_password);
    let server_db_port_env_variable = format!("PHNX_DATABASE_PORT={}", db_port);
    let server_host_env_variable = format!("PHNX_DATABASE_HOST={}", db_domain);
    let server_db_name_env_variable = format!("PHNX_DATABASE_NAME={}", db_name);
    let server_sqlx_offline_env_variable = "SQLX_OFFLINE=true".to_string();

    tracing::info!("Starting phnx server");
    let server_builder = Container::builder(server_image_name, &server_container_name)
        .with_env(&server_domain_env_variable)
        .with_env(&server_host_env_variable)
        .with_env(&server_db_name_env_variable)
        .with_env(&server_db_user_env_variable)
        .with_env(&server_db_password_env_variable)
        .with_env(&server_db_port_env_variable)
        .with_env(&server_sqlx_offline_env_variable)
        .with_network(network_name)
        .with_hostname(server_domain)
        .with_detach(false);

    let server_container = server_builder.build();
    let _server = server_container.run();

    sleep(Duration::from_secs(3));

    tracing::info!("All servers are up, stopping server.");

    // Stop server container
    stop_docker_container(&server_container_name);

    sleep(Duration::from_secs(3));

    tracing::info!("Waited three seconds, starting server again.");

    // Start server container again
    let _server = server_container.run();

    sleep(Duration::from_secs(3));

    stop_docker_container(&server_container_name);
    stop_docker_container(&db_container_name);

    tracing::info!("Done running server restart test");
}

fn docker_exec(container_name: &str, user: &str, args: &[&str]) -> String {
    let output = Command::new("docker")
        .args(["exec", "-u", user, container_name])
        .args(args)
        .output()
        .expect("failed to execute process");

    tracing::info!("Output of docker exec: {:?}", output);

    String::from_utf8(output.stdout).unwrap()
}
