// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use opaque_ke::{ServerLogin, ServerLoginStartParameters, rand::rngs::OsRng};
use phnxtypes::{
    crypto::{OpaqueCiphersuite, opaque::OpaqueLoginResponse},
    errors::auth_service::Init2FactorAuthError,
    messages::client_as::{Init2FactorAuthParamsTbs, Init2FactorAuthResponse},
};

use super::{AuthService, opaque::OpaqueSetup, user_record::UserRecord};

use tls_codec::Serialize;

pub mod anonymous;
pub mod client;
pub mod key_packages;
pub mod privacypass;
pub mod user;

impl AuthService {
    pub(crate) async fn as_init_two_factor_auth(
        &self,
        params: Init2FactorAuthParamsTbs,
    ) -> Result<Init2FactorAuthResponse, Init2FactorAuthError> {
        let Init2FactorAuthParamsTbs {
            client_id,
            opaque_ke1,
        } = params;

        // Load the server setup from storage
        let server_setup = OpaqueSetup::load(&self.db_pool).await.map_err(|e| {
            tracing::error!("Storage provider error: {:?}", e);
            Init2FactorAuthError::StorageError
        })?;

        // Load the user record from storage
        let user_name = &client_id.user_name();
        let password_file_option = UserRecord::load(&self.db_pool, user_name)
            .await
            .map_err(|e| {
                tracing::error!("Error loading user record: {:?}", e);
                Init2FactorAuthError::StorageError
            })?
            .map(|record| record.into_password_file());

        let server_login_result = ServerLogin::<OpaqueCiphersuite>::start(
            &mut OsRng,
            &server_setup,
            password_file_option,
            opaque_ke1.client_message,
            &user_name
                .tls_serialize_detached()
                .map_err(|_| Init2FactorAuthError::LibraryError)?,
            // TODO: We probably want to specify a context, as well as a server
            // and client name here. For now, the default should be okay.
            ServerLoginStartParameters::default(),
        )
        .map_err(|e| {
            tracing::error!("Opaque startup failed with error {e:?}");
            Init2FactorAuthError::OpaqueLoginFailed
        })?;
        let mut client_login_states = self.inner.ephemeral_client_logins.lock().await;
        client_login_states.insert(client_id, server_login_result.state);

        let opaque_login_response = OpaqueLoginResponse {
            server_message: server_login_result.message,
        };
        Ok(Init2FactorAuthResponse {
            opaque_ke2: opaque_login_response,
        })
    }
}
