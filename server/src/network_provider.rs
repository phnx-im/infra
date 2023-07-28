// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::endpoints::{ENDPOINT_QS, ENDPOINT_QS_FEDERATION};
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
    client: Client,
    transport_encryption: TransportEncryption,
}

impl MockNetworkProvider {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            transport_encryption: TransportEncryption::Off,
        }
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
        let url = format!(
            "http{}://{}:8000{}",
            transport_encryption, destination, ENDPOINT_QS_FEDERATION
        );
        // Reqwest should resolve the hostname on its own.
        match self.client.post(url).body(bytes).send().await {
            // For now we don't care about the response.
            Ok(_response) => (),
            // TODO: We only care about the happy path for now.
            Err(e) => panic!("Error: {}", e),
        }
        Ok(())
    }
}
