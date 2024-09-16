// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{
    errors::auth_service::IssueTokensError,
    messages::client_as::{IssueTokensParamsTbs, IssueTokensResponse},
};
use privacypass::batched_tokens_ristretto255::server::Server;

use crate::auth_service::{
    client_record::ClientRecord, privacy_pass::AuthServiceBatchedKeyStoreProvider, AuthService,
};

const MAX_TOKENS_PER_REQUEST: i32 = 100;

impl AuthService {
    pub(crate) async fn as_issue_tokens(
        &self,
        params: IssueTokensParamsTbs,
    ) -> Result<IssueTokensResponse, IssueTokensError> {
        let IssueTokensParamsTbs {
            client_id,
            // This will be used later when we use different token contexts and
            // different challenges for different endpoints.
            token_type: _,
            token_request,
        } = params;
        let tokens_requested = token_request.nr() as i32;

        // Start a transaction
        let mut transaction = self
            .db_pool
            .begin()
            .await
            .map_err(|_| IssueTokensError::StorageError)?;

        // Load current token allowance from storage provider
        let mut client_record = ClientRecord::load(&mut *transaction, &client_id)
            .await
            .map_err(|e| {
                tracing::error!("Error loading client record: {:?}", e);
                IssueTokensError::StorageError
            })?
            .ok_or(IssueTokensError::UnknownClient)?;

        let token_allowance = client_record.token_allowance;
        if tokens_requested > token_allowance || tokens_requested > MAX_TOKENS_PER_REQUEST {
            return Err(IssueTokensError::TooManyTokens);
        }

        let pp_server = Server::new();
        let key_store = AuthServiceBatchedKeyStoreProvider::new(&mut transaction);
        let token_response = pp_server
            .issue_token_response(&key_store, token_request)
            .await
            .map_err(|_| IssueTokensError::PrivacyPassError)?;

        let response = IssueTokensResponse {
            tokens: token_response,
        };

        // Reduce the token allowance by the number of tokens issued.
        client_record.token_allowance -= tokens_requested;
        client_record.update(&mut *transaction).await.map_err(|e| {
            tracing::error!("Error updating client record: {:?}", e);
            IssueTokensError::StorageError
        })?;

        transaction
            .commit()
            .await
            .map_err(|_| IssueTokensError::StorageError)?;

        Ok(response)
    }
}
