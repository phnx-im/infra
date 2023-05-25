pub(crate) mod a_s;
pub(crate) mod d_s;
mod networking;

use super::*;
use ds_lib::{ClientInfo, ClientKeyPackages, GroupMessage};
use tls_codec::{Deserialize, TlsVecU16, TlsVecU32};
use url::Url;

use crate::users::*;

use networking::{get, post};

#[derive(Clone)]
pub struct Backend {
    ds_url: Url,
}

impl Backend {
    /// Create a new backend connection by providing a URL
    pub fn new(url: &str) -> Self {
        Self {
            ds_url: Url::parse(url).unwrap(),
        }
    }
    /// Register a new client with the server.
    pub fn register_client(&self, user: &SelfUser) -> Result<String, String> {
        let mut url = self.ds_url.clone();
        url.set_path("/clients/register");
        let key_package = user.generate_keypackage();

        let client_info = ClientInfo::new(
            user.username.clone(),
            vec![(
                key_package
                    .hash_ref(user.crypto_backend.crypto())
                    .unwrap()
                    .as_slice()
                    .to_vec(),
                key_package.into(),
            )],
        );
        let response = post(&url, &client_info)?;

        Ok(String::from_utf8(response).unwrap())
    }

    /// Get a list of all clients with name, ID, and key packages from the
    /// server.
    pub fn list_clients(&self) -> Result<TlsVecU32<ClientInfo>, String> {
        let mut url = self.ds_url.clone();
        url.set_path("/clients/list");

        let response = get(&url)?;
        match TlsVecU32::<ClientInfo>::tls_deserialize(&mut response.as_slice()) {
            Ok(clients) => Ok(clients),
            Err(e) => Err(format!("Error decoding server response: {:?}", e)),
        }
    }

    /// Get a list of key packages for a client.
    pub fn fetch_key_package(&self, client_id: &[u8]) -> Result<ClientKeyPackages, String> {
        let mut url = self.ds_url.clone();
        let path = "/clients/key_packages/".to_string()
            + &base64::encode_config(client_id, base64::URL_SAFE);
        url.set_path(&path);

        let response = get(&url)?;
        match ClientKeyPackages::tls_deserialize(&mut response.as_slice()) {
            Ok(ckp) => Ok(ckp),
            Err(e) => Err(format!("Error decoding server response: {:?}", e)),
        }
    }

    /// Send a welcome message.
    pub fn send_welcome(&self, welcome_msg: &MlsMessageOut) -> Result<(), String> {
        let mut url = self.ds_url.clone();
        url.set_path("/send/welcome");

        // The response should be empty.
        let _response = post(&url, welcome_msg)?;
        Ok(())
    }

    /// Send a group message.
    pub fn send_msg(&self, group_msg: &GroupMessage) -> Result<(), String> {
        // The server doesn't like empty recipient lists.
        if group_msg.recipients.is_empty() {
            println!("send_msg: Empty recipient list.");
            return Ok(());
        }
        let mut url = self.ds_url.clone();
        url.set_path("/send/message");

        // The response should be empty.
        let _response = post(&url, group_msg)?;
        Ok(())
    }

    /// Get a list of all new messages for the user.
    pub fn recv_msgs(&self, credential: &Credential) -> Result<TlsVecU16<MlsMessageIn>, String> {
        let mut url = self.ds_url.clone();
        let path =
            "/recv/".to_string() + &base64::encode_config(credential.identity(), base64::URL_SAFE);
        url.set_path(&path);

        let response = get(&url)?;
        TlsVecU16::<MlsMessageIn>::tls_deserialize(&mut response.as_slice())
            .map_err(|e| format!("Invalid message list: {:?}", e))
    }

    /// Reset the DS.
    pub fn reset_backend(&self) -> Result<(), String> {
        let mut url = self.ds_url.clone();
        url.set_path("reset");
        match get(&url) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Error resetting the backend: {:?}", e)),
        }
    }
}

impl Default for Backend {
    fn default() -> Self {
        Self {
            ds_url: Url::parse("http://127.0.0.1:8080").unwrap(),
        }
    }
}
