// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use tokio::process::{Child, Command};

use phnxapiclient::{ApiClient, TransportEncryption};
use phnxbackend::qs::Fqdn;

async fn build_docker_image(path_to_docker_file: &str, image_name: &str) {
    tracing::info!("Building docker image: {}", image_name);
    let build_output = Command::new("docker")
        .arg("build")
        .arg("-t")
        .arg(image_name)
        .arg("-f")
        .arg(path_to_docker_file)
        .arg(".")
        .output()
        .await
        .expect("failed to execute process");

    let command_stdout = String::from_utf8(build_output.stdout).unwrap();
    let command_stderr = String::from_utf8(build_output.stderr).unwrap();

    tracing::info!("Run output: {:?}, {:?}", command_stdout, command_stderr);
}

async fn run_docker_container(
    image_name: &str,
    env_variables: &[&str],
    hostname_option: Option<&str>,
    network_name_option: Option<&str>,
) -> Child {
    tracing::info!("Passing in env variables: {:?}", env_variables);
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
    command.args(["--rm", image_name]);
    command.spawn().unwrap()
}

pub(crate) async fn create_and_start_server_container(
    server_domain: Fqdn,
    network_name_option: Option<&str>,
) -> Child {
    // First go into the workspace dir s.t. we can build the docker image.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    std::env::set_current_dir(manifest_dir.to_owned() + "/..").unwrap();

    let image_name = "phnxserver_image";

    build_docker_image("server/Dockerfile", &image_name).await;

    let server_domain_env_variable = format!("PHNX_SERVER_DOMAIN={}", server_domain);
    run_docker_container(
        &image_name,
        &[&server_domain_env_variable],
        Some(&server_domain.to_string()),
        network_name_option,
    )
    .await
}

pub(crate) async fn create_and_start_test_container(
    test_name: &str,
    network_name_option: Option<&str>,
) {
    // First go into the workspace dir s.t. we can build the docker image.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    std::env::set_current_dir(manifest_dir.to_owned() + "/..").unwrap();

    let image_name = format!("{}_test_image", test_name);

    build_docker_image("test_harness/Dockerfile", &image_name).await;

    let test_scenario_env_variable = format!("PHNX_TEST_SCENARIO={}", test_name);

    run_docker_container(
        &image_name,
        &[&test_scenario_env_variable, "TEST_LOG=true"],
        // No hostname required for the test container
        None,
        network_name_option,
    )
    .await;

    // TODO: The above works. The next steps are as follows:
    // * Run a test involving just the harness and the server
    // * Figure out a network and a DNS configuration to make it work
    // * Run a test involving two servers and the harness.
}

pub(crate) async fn create_network(network_name: &str) {
    tracing::info!("Creating network: {}", network_name);
    let command_output = Command::new("docker")
        .arg("network")
        .arg("create")
        .arg(network_name)
        .output()
        .await
        .expect("failed to execute process");

    let command_stdout = String::from_utf8(command_output.stdout).unwrap();
    let command_stderr = String::from_utf8(command_output.stderr).unwrap();

    tracing::info!("Run output: {:?}, {:?}", command_stdout, command_stderr);
}
