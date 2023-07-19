// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{collections::HashMap, sync::Mutex};

use crate::endpoints::ENDPOINT_QS;
use async_trait::async_trait;
use phnxbackend::qs::{network_provider_trait::NetworkProvider, Fqdn};
use reqwest::Client;
use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum MockNetworkError {}

#[derive(Debug)]
pub enum TransportEncryption {
    On,
    Off,
}

#[derive(Debug)]
pub struct MockNetworkProvider {
    backend_ports: Mutex<HashMap<Fqdn, u16>>,
    client: Client,
    transport_encryption: TransportEncryption,
}

impl MockNetworkProvider {
    pub fn new() -> Self {
        Self {
            backend_ports: Mutex::new(HashMap::new()),
            client: Client::new(),
            transport_encryption: TransportEncryption::Off,
        }
    }

    pub fn add_port(&self, fqdn: Fqdn, port: u16) {
        self.backend_ports.lock().unwrap().insert(fqdn, port);
    }
}

#[async_trait]
impl NetworkProvider for MockNetworkProvider {
    type NetworkError = MockNetworkError;

    async fn deliver(&self, bytes: Vec<u8>, destination: Fqdn) -> Result<(), Self::NetworkError> {
        let transport_encryption = match self.transport_encryption {
            TransportEncryption::On => "s",
            TransportEncryption::Off => "",
        };
        tracing::info!("Currently registered {:?}", self.backend_ports);
        tracing::info!("Sending to {:?}", destination);
        let port = self
            .backend_ports
            .lock()
            .unwrap()
            .get(&destination)
            .unwrap()
            .to_owned();
        // For now, we don't resolve the actual hostname and just send to
        // localhost.
        let destination = "localhost";
        let url = format!(
            "http{}://{}:{}{}",
            transport_encryption, destination, port, ENDPOINT_QS
        );
        match self.client.post(url).body(bytes).send().await {
            // For now we don't care about the response.
            Ok(_response) => (),
            // TODO: We only care about the happy path for now.
            Err(e) => panic!("Error: {}", e),
        }
        Ok(())
    }
}
