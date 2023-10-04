// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::messages::client_as::{IssueTokensParamsTbs, IssueTokensResponse};
use privacypass::batched_tokens::server::Server;

use crate::auth_service::{
    errors::IssueTokensError, storage_provider_trait::AsStorageProvider, AuthService,
};

const MAX_TOKENS_PER_REQUEST: usize = 100;

impl AuthService {
    pub(crate) async fn as_issue_tokens<S: AsStorageProvider>(
        storage_provider: &S,
        params: IssueTokensParamsTbs,
    ) -> Result<IssueTokensResponse, IssueTokensError> {
        let IssueTokensParamsTbs {
            client_id,
            // This will be used later when we use different token contexts and
            // different challenges for different endpoints.
            token_type: _,
            token_request,
        } = params;

        // Load current token allowance from storage provider
        let token_allowance = storage_provider
            .load_client_token_allowance(&client_id)
            .await
            .map_err(|_| IssueTokensError::StorageError)?;

        let tokens_requested = token_request.nr();
        if tokens_requested > token_allowance || tokens_requested > MAX_TOKENS_PER_REQUEST {
            return Err(IssueTokensError::TooManyTokens);
        }

        let pp_key_store = storage_provider.privacy_pass_key_store().await;
        let pp_server = Server::new();
        let token_response = pp_server
            .issue_token_response(pp_key_store, token_request)
            .await
            .map_err(|_| IssueTokensError::PrivacyPassError)?;

        let response = IssueTokensResponse {
            tokens: token_response,
        };

        // Reduce the token allowance by the number of tokens issued.
        storage_provider
            .set_client_token_allowance(&client_id, token_allowance - tokens_requested)
            .await
            .map_err(|_| IssueTokensError::StorageError)?;

        Ok(response)
    }
}
