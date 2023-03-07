/*
Endpoints:
 - ENDPOINT_QS_CREATE_USER_RECORD
 - ENDPOINT_QS_UPDATE_USER_RECORD
 - ENDPOINT_QS_USER_RECORD
 - ENDPOINT_QS_DELETE_USER_RECORD
*/

use crate::{
    crypto::RatchetKey,
    messages::client_qs::{
        CreateUserRecordParams, CreateUserRecordResponse, UpdateUserRecordParams,
    },
    qs::{
        client_record::QsClientRecord,
        errors::{QsCreateUserError, QsUpdateUserError},
        storage_provider_trait::QsStorageProvider,
        user_record::QsUserRecord,
        Qs,
    },
};

impl Qs {
    /// Update the info of a given queue. Requires a valid signature by the
    /// owner of the queue.
    #[tracing::instrument(skip_all, err)]
    pub async fn qs_create_user_record<S: QsStorageProvider>(
        storage_provider: &S,
        params: CreateUserRecordParams,
    ) -> Result<CreateUserRecordResponse, QsCreateUserError<S>> {
        let CreateUserRecordParams {
            user_record_auth_key,
            friendship_token,
            client_record_auth_key,
            queue_encryption_key,
        } = params;

        // TODO: Signature must be verified.

        let seed = rand::random::<[u8; 32]>();

        // TODO: The ratchet key must be KEMed to the client using
        // `queue_encryption_key`.
        let ratchet_key = RatchetKey::new(seed.to_vec());

        let (client_record, client_id) = QsClientRecord::try_new(
            storage_provider,
            None,
            queue_encryption_key,
            client_record_auth_key,
            ratchet_key,
        )
        .await
        .map_err(|e| QsCreateUserError::ClientCreationError(e))?;

        let user_record =
            QsUserRecord::new(user_record_auth_key, friendship_token, client_id.clone());

        tracing::trace!("Storing QsUserProfile in storage provider");
        let user_id = storage_provider
            .create_user(&user_record)
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                QsCreateUserError::StorageProviderError(e)
            })?;

        let response = CreateUserRecordResponse {
            user_id,
            client_id,
            client_record,
        };

        Ok(response)
    }

    /// Update a user record.
    #[tracing::instrument(skip_all, err)]
    pub async fn qs_update_user_record<S: QsStorageProvider>(
        storage_provider: &S,
        params: UpdateUserRecordParams,
    ) -> Result<(), QsUpdateUserError> {
        let UpdateUserRecordParams {
            user_id,
            user_record_auth_key,
            friendship_token,
        } = params;

        let mut user_record = storage_provider
            .load_user(&user_id)
            .await
            .ok_or(QsUpdateUserError::StorageError)?;

        user_record.update(user_record_auth_key, friendship_token);

        storage_provider
            .store_user(&user_id, user_record)
            .await
            .map_err(|_| QsUpdateUserError::StorageError)?;
        todo!()
    }
}
