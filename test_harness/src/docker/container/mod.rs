// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::process::{Child, Command};

pub mod builder;

pub(super) struct Container {
    image: String,
    name: String,
    env: Vec<String>,
    hostname: Option<String>,
    network: Option<String>,
    port: Option<String>,
    run_parameters: Vec<String>,
    detach: bool,
    volumes: Vec<String>,
}

impl Container {
    pub(super) fn builder(image: &str, name: &str) -> builder::ContainerBuilder {
        builder::ContainerBuilder::new(image, name)
    }

    pub(super) fn run(&self) -> Child {
        let mut command = Command::new("docker");
        command.arg("run");
        for env_variable in &self.env {
            command.args(["--env", env_variable]);
        }
        for volume in &self.volumes {
            command.args(["--volume", volume]);
        }
        if let Some(network_name) = &self.network {
            command.args(["--network", network_name]);
        }
        if let Some(hostname) = &self.hostname {
            command.args(["--hostname", hostname]);
        }
        command.args(["--name", &self.name]);
        if let Some(port) = &self.port {
            command.args(["-p", port.to_string().as_str()]);
        }
        command.args(["--rm"]);
        if self.detach {
            command.args(["-d"]);
        }
        command.args([&self.image]);
        command.args(&self.run_parameters);
        command.spawn().unwrap()
    }
}
