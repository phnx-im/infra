//! API endpoints of the DS

use super::*;
use mls_assist::GroupId;
use phnxbackend::{
    ds::{api::DsProcessResponse, errors::DsProcessingError},
    messages::client_ds::{ClientToDsMessage, CreateGroupParams, VerifiableClientToDsMessage},
};
use phnxserver::endpoints::ENDPOINT_DS;

#[cfg(test)]
mod tests;

#[derive(Error, Debug)]
pub enum DsCreateGroupError {
    #[error("Bad request")]
    BadRequest,
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
}

impl ApiClient {
    /// Creates a new group on the DS.
    pub async fn process_ds_message(
        &self,
        message: ClientToDsMessage,
    ) -> Result<DsProcessResponse, DsProcessingError> {
        // TODO: Before we can continue here, we need to define an -In vs. -Out
        // variant for all the DS message structs.
        let message_bytes = todo!();
        //match self
        //    .client
        //    .post(self.build_url(Protocol::Http, ENDPOINT_DS))
        //    .body(body)
        //    .json(&create_group_params)
        //    .send()
        //    .await
        //{
        //    Ok(res) => {
        //        let group_id = res.json::<GroupId>().await?;
        //        Ok(group_id)
        //    }
        //    Err(err) => {
        //        if let Some(status_code) = err.status() {
        //            if status_code == 400 {
        //                Err(DsCreateGroupError::BadRequest)
        //            } else {
        //                Err(err.into())
        //            }
        //        } else {
        //            Err(err.into())
        //        }
        //    }
        //}
    }
}
