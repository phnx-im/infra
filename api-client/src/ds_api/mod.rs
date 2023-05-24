// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! API endpoints of the DS

use super::*;
use mls_assist::{
    messages::AssistedWelcome, treesync::RatchetTree, Extensions, GroupInfo, MlsMessageOut,
    Signature, TlsDeserializeTrait, TlsSerializeTrait,
};
use phnxbackend::{
    crypto::{
        ear::keys::GroupStateEarKey,
        signatures::{keys::UserAuthSigningKey, signable::Signable, traits::SigningKey},
    },
    ds::{errors::DsProcessingError, group_state::EncryptedCredentialChain},
    messages::client_ds_out::{
        AddUsersParamsOut, ClientToDsMessageTbsOut, CreateGroupParamsOut, DsProcessResponseIn,
        DsRequestParamsOut,
    },
    qs::{KeyPackageBatch, QsClientReference, VERIFIED},
};
use phnxserver::endpoints::ENDPOINT_DS;

#[cfg(test)]
mod tests;

#[derive(Error, Debug)]
pub enum DsRequestError {
    #[error("Library Error")]
    LibraryError,
    #[error("Couldn't deserialize response body.")]
    BadResponse,
    #[error("Network error")]
    NetworkError(String),
    #[error(transparent)]
    DsError(#[from] DsProcessingError),
}

impl ApiClient {
    async fn prepare_and_send_message(
        &self,
        request_params: DsRequestParamsOut,
        signing_key: &impl SigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<DsProcessResponseIn, DsRequestError> {
        let tbs = ClientToDsMessageTbsOut::new(group_state_ear_key.clone(), request_params);
        let message = tbs
            .sign(signing_key)
            .map_err(|_| DsRequestError::LibraryError)?;
        let message_bytes = message
            .tls_serialize_detached()
            .map_err(|_| DsRequestError::LibraryError)?;
        match self
            .client
            .post(self.build_url(Protocol::Http, ENDPOINT_DS))
            .body(message_bytes)
            .send()
            .await
        {
            Ok(res) => {
                match res.status().as_u16() {
                    // Success!
                    x if (200..=299).contains(&x) => {
                        let ds_proc_res_bytes =
                            res.bytes().await.map_err(|_| DsRequestError::BadResponse)?;
                        let ds_proc_res =
                            DsProcessResponseIn::tls_deserialize_bytes(ds_proc_res_bytes)
                                .map_err(|_| DsRequestError::BadResponse)?;
                        Ok(ds_proc_res)
                    }
                    // DS Specific Error
                    418 => {
                        let ds_proc_err_bytes =
                            res.bytes().await.map_err(|_| DsRequestError::BadResponse)?;
                        let ds_proc_err =
                            DsProcessingError::tls_deserialize_bytes(ds_proc_err_bytes)
                                .map_err(|_| DsRequestError::BadResponse)?;
                        Err(DsRequestError::DsError(ds_proc_err))
                    }
                    // All other errors
                    _ => {
                        let error_text =
                            res.text().await.map_err(|_| DsRequestError::BadResponse)?;
                        Err(DsRequestError::NetworkError(error_text))
                    }
                }
            }
            // A network error occurred.
            Err(err) => Err(DsRequestError::NetworkError(err.to_string())),
        }
    }

    /// Creates a new group on the DS.
    pub async fn ds_create_group(
        &self,
        leaf_node: RatchetTree,
        encrypted_credential_chain: EncryptedCredentialChain,
        creator_client_reference: QsClientReference,
        group_info: GroupInfo,
        group_state_ear_key: &GroupStateEarKey,
        signing_key: &UserAuthSigningKey,
    ) -> Result<DsProcessResponseIn, DsRequestError> {
        let payload = CreateGroupParamsOut {
            group_id: group_info.group_context().group_id().clone(),
            leaf_node,
            encrypted_credential_chain,
            creator_client_reference,
            group_info,
            creator_user_auth_key: signing_key.verifying_key().clone(),
        };
        self.prepare_and_send_message(
            DsRequestParamsOut::CreateGroupParams(payload),
            signing_key,
            group_state_ear_key,
        )
        .await
    }

    /// Add a user to a group.
    pub async fn ds_add_user(
        &self,
        commit: (MlsMessageOut, (Signature, Extensions)),
        welcome: AssistedWelcome,
        encrypted_welcome_attribution_infos: Vec<Vec<u8>>,
        key_package_batches: Vec<KeyPackageBatch<VERIFIED>>,
        group_state_ear_key: &GroupStateEarKey,
        signing_key: &UserAuthSigningKey,
    ) -> Result<DsProcessResponseIn, DsRequestError> {
        let payload = AddUsersParamsOut {
            commit,
            sender: signing_key.verifying_key().hash(),
            welcome,
            encrypted_welcome_attribution_infos,
            key_package_batches,
        };
        self.prepare_and_send_message(
            DsRequestParamsOut::AddUsers(payload),
            signing_key,
            group_state_ear_key,
        )
        .await
    }
}
