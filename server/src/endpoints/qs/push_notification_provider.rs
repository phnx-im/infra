// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use async_trait::async_trait;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use phnxbackend::qs::{PushNotificationError, PushNotificationProvider};
use phnxtypes::messages::push_token::{PushToken, PushTokenOperator};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::Read,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::configurations::ApnsSettings;

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
pub struct ApnsState {
    pub key_id: String,
    pub team_id: String,
    pub private_key: Vec<u8>,
    token: Arc<Mutex<Option<ApnsToken>>>,
}

#[derive(Debug, Clone)]
pub struct ProductionPushNotificationProvider {
    apns_state: Option<ApnsState>,
}

impl ProductionPushNotificationProvider {
    // Create a new ProductionPushNotificationProvider. If the config_option is
    // None, the provider will effectively not send push notifications.
    pub fn new(config_option: Option<ApnsSettings>) -> Result<Self, Box<dyn std::error::Error>> {
        let Some(config) = config_option else {
            return Ok(Self { apns_state: None });
        };
        // Read the private key
        let mut private_key_file = File::open(&config.privatekeypath)?;
        let mut private_key_p8 = String::new();
        private_key_file.read_to_string(&mut private_key_p8)?;

        Ok(Self {
            apns_state: Some(ApnsState {
                key_id: config.keyid,
                team_id: config.teamid,
                private_key: private_key_p8.as_bytes().to_vec(),
                token: Arc::new(Mutex::new(None)),
            }),
        })
    }

    /// Return the JWT. If the token is older than 40 minutes, a new token is
    /// issued (as JWTs must be between 20 and 60 minutes old).
    fn issue_jwt(&self) -> Result<String, Box<dyn std::error::Error>> {
        let apns_state = self.apns_state.as_ref().ok_or("Missing ApnsState")?;

        // Check whether we already have a token and if it is still valid, i.e.
        // not older than 40 minutes
        let mut token_option = apns_state
            .token
            .lock()
            .map_err(|_| "Could not lock token mutex")?;

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
            iss: apns_state.team_id.clone(),
            iat,
        };

        // Create the JWT header
        let mut header = Header::new(Algorithm::ES256);
        header.kid = Some(apns_state.key_id.clone());

        // Encode the JWT
        let token = encode(
            &header,
            &claims,
            &EncodingKey::from_ec_pem(&apns_state.private_key)?,
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
                // If we don't have an APNS state, we can't send push notifications
                if self.apns_state.is_none() {
                    return Ok(());
                }

                // Issue the JWT
                let jwt = self
                    .issue_jwt()
                    .map_err(|e| PushNotificationError::JwtCreationError(e.to_string()))?;

                // Create the URL
                let url = format!(
                    "https://api.push.apple.com:443/3/device/{}",
                    push_token.token()
                );

                // Create the headers and payload
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert("authorization", format!("bearer {}", jwt).parse().unwrap());
                headers.insert("apns-topic", "im.phnx.prototype".parse().unwrap());
                headers.insert("apns-push-type", "alert".parse().unwrap());
                headers.insert("apns-priority", "10".parse().unwrap());
                headers.insert("apns-expiration", "0".parse().unwrap());

                let body = r#"
                {
                    "aps": {
                        "alert": {
                        "title": "Empty notification",
                        "body": "Please report this issue"
                        },
                         "mutable-content": 1
                    },
                    "data": "data",
                }
                "#;

                // Send the push notification
                let client = Client::new();
                let res = client
                    .post(url)
                    .headers(headers)
                    .body(body)
                    .send()
                    .await
                    .map_err(|e| PushNotificationError::NetworkError(e.to_string()))?;

                match res.status() {
                    StatusCode::OK => Ok(()),
                    // If the token is invalid, we might want to know it and
                    // delete it
                    StatusCode::GONE => Err(PushNotificationError::InvalidToken(
                        res.text().await.unwrap_or_default(),
                    )),
                    // If the status code is not OK or GONE, we might want to
                    // log the error
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
