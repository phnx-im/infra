// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;

use bollard::{
    container::{Config, CreateContainerOptions, LogsOptions},
    image::BuildImageOptions,
    Docker,
};
use phnxserver_test_harness::run_federation_scenario;
use tokio_stream::StreamExt;

async fn create_and_start_server_container(docker: &Docker, domain: &str) {
    // TODO: Pass configuration such as domain into this function and the set
    // them as environment variables. When starting the server, read the
    // variables from the environment.
    let server_build_options = BuildImageOptions {
        dockerfile: "server.Dockerfile",
        t: "phnxserver_image",
        ..Default::default()
    };
    let mut server_image_build_stream = docker.build_image(server_build_options, None, None);

    while let Some(msg) = server_image_build_stream.next().await {
        tracing::info!("Message: {:?}", msg);
    }

    // TODO: This is where we set the domain name (host name?) later
    let server_container_config = Config {
        image: Some("phnxserver_image"),
        ..Default::default()
    };
    let container_name = format!("{}_phnxserver", domain);

    let server_create_container_options = CreateContainerOptions {
        name: container_name,
        platform: Some("linux/amd64".to_owned()),
    };

    docker
        .create_container(
            Some(server_create_container_options),
            server_container_config,
        )
        .await
        .unwrap();

    // await here returns once the container is ready (?)
    docker
        .start_container::<String>("phnxserver_container", None)
        .await
        .unwrap();
}

async fn create_and_start_test_container(docker: &Docker, test_name: &str) {
    tracing::info!("Creating and starting test container");
    let binary_path = "";
    tracing::info!("Binary path: {}", binary_path);
    tracing::info!("Test name: {}", test_name);

    let build_args: HashMap<&str, &str> =
        [("name", test_name), ("binary_path", &binary_path)].into();
    let test_build_options = BuildImageOptions {
        dockerfile: "tests/test.Dockerfile",
        t: "connect_federated_users_test_image",
        buildargs: build_args,
        ..Default::default()
    };
    let mut test_image_build_stream = docker.build_image(test_build_options, None, None);

    while let Some(msg) = test_image_build_stream.next().await {
        tracing::info!("Message: {:?}", msg);
    }

    // TODO: This is where we set the domain name (host name?) later
    //let test_container_config = Config {
    //    image: Some("connect_federated_users_test_image"),
    //    ..Default::default()
    //};

    //let test_create_container_options = CreateContainerOptions {
    //    name: "connect_federated_users_test_container",
    //    platform: Some("linux/amd64"),
    //};

    //docker
    //    .create_container(
    //        Some(server_create_container_options),
    //        server_container_config,
    //    )
    //    .await
    //    .unwrap();

    //// await here returns once the container is ready (?)
    //docker
    //    .start_container::<String>("connect_federated_users_test_container", None)
    //    .await
    //    .unwrap();
}

#[actix_rt::test]
#[tracing::instrument(name = "Connect federated users test", skip_all)]
async fn connect_federated_users() {
    run_federation_scenario().await;
}
