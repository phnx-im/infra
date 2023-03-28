// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use tls_codec::{TlsDeserialize, TlsSerialize, TlsSize};

use crate::{
    auth_service::{
        errors::IssueTokensError, storage_provider_trait::AsStorageProvider, AuthService,
    },
    messages::client_as::{IssueTokensParams, IssueTokensResponse},
};

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
#[repr(u8)]
pub enum AsTokenType {
    AsEnqueue,
    AsKeyPackageBatch,
    DsGroupCreation,
    DsGroupOperation,
    QsKeyPackageBatch,
}

impl AuthService {
    pub async fn as_issue_tokens<S: AsStorageProvider>(
        &self,
        storage_provider: &S,
        params: IssueTokensParams,
    ) -> Result<IssueTokensResponse, IssueTokensError> {
        let IssueTokensParams {
            auth_method,
            token_type,
            token_request,
        } = params;

        todo!()
    }
}
