// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use async_trait::async_trait;
use base64::{engine::general_purpose, Engine};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use phnxbackend::qs::{PushNotificationError, PushNotificationProvider};
use phnxtypes::messages::push_token::{PushToken, PushTokenOperator};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    key_id: String,
    team_id: String,
    private_key_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    iss: String,
    iat: usize,
}

#[derive(Debug, Clone)]
pub struct ApnsToken {
    jwt: String,
    issued_at: u64,
}

#[derive(Debug, Clone)]
pub struct ProductionPushNotificationProvider {
    team_id: String,
    key_id: String,
    private_key: Vec<u8>,
    token: Arc<Mutex<Option<ApnsToken>>>,
}

impl ProductionPushNotificationProvider {
    pub fn new(config_file_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Read the config file
        let mut file = File::open(config_file_path)?;
        let mut config_data = String::new();
        file.read_to_string(&mut config_data)?;
        let config: Config = serde_json::from_str(&config_data)?;

        // Read the private key
        let mut private_key_file = File::open(&config.private_key_path)?;
        let mut private_key_p8 = String::new();
        private_key_file.read_to_string(&mut private_key_p8)?;

        // The private key needs to be converted to the correct format (PEM format)
        let pem = private_key_p8
            .replace("-----BEGIN PRIVATE KEY-----", "")
            .replace("-----END PRIVATE KEY-----", "")
            .replace("\n", "")
            .replace("\r", "");

        // Convert the private key to bytes
        let private_key = general_purpose::STANDARD.decode(&pem)?;

        Ok(Self {
            key_id: config.key_id,
            team_id: config.team_id,
            private_key,
            token: Arc::new(Mutex::new(None)),
        })
    }

    /// Return the JWT. If the token is older than 40 minutes, a new token is
    /// issued (as JWTs must be between 20 and 60 minutes old).
    fn issue_jwt(&self) -> Result<String, Box<dyn std::error::Error>> {
        // Check whether we already have a token and if it is still valid, i.e.
        // not older than 40 minutes
        let mut token_option = self.token.lock().map_err(|_| "error")?;

        if let Some(token) = &*token_option {
            let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
            if now < token.issued_at + 60 * 40 {
                return Ok(token.jwt.clone());
            }
        }
        // Generate the current time in seconds since UNIX_EPOCH
        let iat = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as usize;

        // Create the JWT claims
        let claims = Claims {
            iss: self.team_id.clone(),
            iat,
        };

        // Create the JWT header
        let mut header = Header::new(Algorithm::ES256);
        header.kid = Some(self.key_id.clone());

        // Encode the JWT
        let token = encode(
            &header,
            &claims,
            &EncodingKey::from_ec_pem(&self.private_key)?,
        )?;

        // Store the JWT and update the last issuance time
        *token_option = Some(ApnsToken {
            jwt: token.clone(),
            issued_at: iat as u64,
        });

        Ok(token)
    }
}

#[async_trait]
impl PushNotificationProvider for ProductionPushNotificationProvider {
    async fn push(&self, push_token: PushToken) -> Result<(), PushNotificationError> {
        match push_token.operator() {
            PushTokenOperator::Apple => {
                let url = format!(
                    "https://api.push.apple.com:443/3/device/{}",
                    push_token.token()
                );

                let mut headers = reqwest::header::HeaderMap::new();
                let jwt = self
                    .issue_jwt()
                    .map_err(|e| PushNotificationError::JwtCreationError(e.to_string()))?;
                headers.insert("authorization", format!("bearer {}", jwt).parse().unwrap());
                headers.insert("apns-topic", "im.phnx.prototype".parse().unwrap());
                headers.insert("apns-push-type", "alert".parse().unwrap());
                headers.insert("apns-priority", "10".parse().unwrap());
                headers.insert("apns-expiration", "0".parse().unwrap());

                let mut payload = HashMap::new();
                payload.insert(
                    "aps",
                    serde_json::json!({
                        "alert": {
                            "title": "Prototype",
                            "body": "placeholder body"
                        },
                        "mutable-content": 1
                    }),
                );
                payload.insert("customData", serde_json::json!("custom payload"));

                let client = Client::new();
                let res = client
                    .post(url)
                    .headers(headers)
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| PushNotificationError::NetworkError(e.to_string()))?;

                match res.status() {
                    StatusCode::OK => Ok(()),
                    StatusCode::GONE => Err(PushNotificationError::InvalidToken(
                        res.text().await.unwrap_or_default(),
                    )),
                    s => Err(PushNotificationError::Other(format!(
                        "Unexpected status code: {} with body: {}",
                        s,
                        res.text().await.unwrap_or_default()
                    ))),
                }
            }
            PushTokenOperator::Google => Err(PushNotificationError::UnsupportedType),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TestPushTokenProvider {}

#[async_trait]
impl PushNotificationProvider for TestPushTokenProvider {
    async fn push(&self, _push_token: PushToken) -> Result<(), PushNotificationError> {
        Ok(())
    }
}
