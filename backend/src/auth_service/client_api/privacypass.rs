// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxcommon::identifiers::UserId;
use privacypass::{
    amortized_tokens::{AmortizedBatchTokenRequest, AmortizedBatchTokenResponse, server::Server},
    private_tokens::Ristretto255,
};
use tracing::error;

use crate::{
    auth_service::{
        AuthService, client_record::ClientRecord, privacy_pass::AuthServiceBatchedKeyStoreProvider,
    },
    errors::auth_service::IssueTokensError,
};

const MAX_TOKENS_PER_REQUEST: i32 = 100;

impl AuthService {
    pub(crate) async fn as_issue_tokens(
        &self,
        user_id: &UserId,
        token_request: AmortizedBatchTokenRequest<Ristretto255>,
    ) -> Result<AmortizedBatchTokenResponse<Ristretto255>, IssueTokensError> {
        let tokens_requested = token_request.nr() as i32;

        // Start a transaction
        let mut transaction = self
            .db_pool
            .begin()
            .await
            .map_err(|_| IssueTokensError::StorageError)?;

        // Load current token allowance from storage provider
        let mut client_record = ClientRecord::load(&mut *transaction, user_id)
            .await
            .map_err(|error| {
                error!(%error, "Error loading client record");
                IssueTokensError::StorageError
            })?
            .ok_or(IssueTokensError::UnknownUser)?;

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

        // Reduce the token allowance by the number of tokens issued.
        client_record.token_allowance -= tokens_requested;
        client_record.update(&mut *transaction).await.map_err(|e| {
            error!("Error updating client record: {:?}", e);
            IssueTokensError::StorageError
        })?;

        transaction
            .commit()
            .await
            .map_err(|_| IssueTokensError::StorageError)?;

        Ok(token_response)
    }
}
