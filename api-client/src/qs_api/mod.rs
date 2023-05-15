use phnxbackend::messages::QueueMessage;
use thiserror::Error;

use crate::ApiClient;

pub mod ws;

// TODO: No tests for now.
//#[cfg(test)]
//mod tests;

#[derive(Error, Debug)]
pub enum QsRequestError {
    #[error("Bad request")]
    BadRequest,
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
}

impl ApiClient {
    /// Send message
    pub async fn qs_send_messages(
        &self,
        //message: ClientToQsMessage,
    ) -> Result<(Vec<QueueMessage>, u64), QsRequestError> {
        // TODO: Before we can continue here, we need to define the necessary
        // -In vs. -Out types for the message structs.
        todo!()
        //match self
        //    .client
        //    .post(&format!("{}{}", self.base_url, ENDPOINT_QS_FETCH_MESSAGES))
        //    .json(&fetch_message_params)
        //    .send()
        //    .await
        //{
        //    Ok(res) => {
        //        let (messages, messages_left) = res.json::<(Vec<EnqueuedMessage>, u64)>().await?;
        //        Ok((messages, messages_left))
        //    }
        //    Err(err) => {
        //        if let Some(status_code) = err.status() {
        //            if status_code == 400 {
        //                Err(QsRequestError::BadRequest)
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
