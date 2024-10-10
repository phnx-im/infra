// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::Container;

pub struct ContainerBuilder {
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

impl ContainerBuilder {
    pub fn new(image: &str, name: &str) -> Self {
        Self {
            image: image.to_string(),
            name: name.to_string(),
            env: Vec::new(),
            hostname: None,
            network: None,
            port: None,
            run_parameters: Vec::new(),
            detach: false,
            volumes: Vec::new(),
        }
    }

    pub fn with_env(mut self, env: &str) -> Self {
        self.env.push(env.to_string());
        self
    }

    pub fn with_hostname(mut self, hostname: &str) -> Self {
        self.hostname = Some(hostname.to_string());
        self
    }

    pub fn with_network(mut self, network: &str) -> Self {
        self.network = Some(network.to_string());
        self
    }

    pub fn with_port(mut self, port: &str) -> Self {
        self.port = Some(port.to_string());
        self
    }

    pub fn with_run_parameters(mut self, parameters: &[&str]) -> Self {
        self.run_parameters
            .extend(parameters.iter().map(|p| p.to_string()));
        self
    }

    pub fn with_detach(mut self, detach: bool) -> Self {
        self.detach = detach;
        self
    }

    pub fn with_volume(mut self, volume: &str) -> Self {
        self.volumes.push(volume.to_string());
        self
    }

    pub fn build(self) -> Container {
        Container {
            image: self.image,
            name: self.name,
            env: self.env,
            hostname: self.hostname,
            network: self.network,
            port: self.port,
            run_parameters: self.run_parameters,
            detach: self.detach,
            volumes: self.volumes,
        }
    }
}
