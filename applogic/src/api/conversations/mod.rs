use anyhow::{anyhow, Result};
use phnxtypes::identifiers::{SafeTryInto, UserName};

use super::{
    types::{ConversationIdBytes, UiContact, UiConversation},
    User,
};

impl User {
    pub async fn get_conversations(&self) -> Vec<UiConversation> {
        let user = self.user.lock().await;
        user.conversations()
            .unwrap_or_default()
            .into_iter()
            .map(|c| c.into())
            .collect()
    }

    #[tokio::main(flavor = "current_thread")]
    pub async fn create_conversation(&self, name: String) -> Result<ConversationIdBytes> {
        let mut user = self.user.lock().await;
        Ok(ConversationIdBytes::from(
            user.create_conversation(&name, None).await?,
        ))
    }

    pub async fn set_conversation_picture(
        &self,
        conversation_id: ConversationIdBytes,
        conversation_picture: Option<Vec<u8>>,
    ) -> Result<()> {
        let user = self.user.lock().await;
        user.set_conversation_picture(conversation_id.into(), conversation_picture)?;
        Ok(())
    }

    #[tokio::main(flavor = "current_thread")]
    pub async fn add_users_to_conversation(
        &self,
        conversation_id: ConversationIdBytes,
        user_names: Vec<String>,
    ) -> Result<()> {
        let mut user = self.user.lock().await;
        let conversation_messages = user
            .invite_users(
                conversation_id.into(),
                &user_names
                    .into_iter()
                    .map(|s| <String as SafeTryInto<UserName>>::try_into(s))
                    .collect::<Result<Vec<UserName>, _>>()?,
            )
            .await?;
        self.dispatch_message_notifications(conversation_messages)
            .await;
        Ok(())
    }

    #[tokio::main(flavor = "current_thread")]
    pub async fn remove_users_from_conversation(
        &self,
        conversation_id: ConversationIdBytes,
        user_names: Vec<String>,
    ) -> Result<()> {
        let mut user = self.user.lock().await;
        let conversation_messages = user
            .remove_users(
                conversation_id.into(),
                &user_names
                    .into_iter()
                    .map(|s| <String as SafeTryInto<UserName>>::try_into(s))
                    .collect::<Result<Vec<UserName>, _>>()?,
            )
            .await?;
        self.dispatch_message_notifications(conversation_messages)
            .await;
        Ok(())
    }

    pub async fn members_of_conversation(
        &self,
        conversation_id: ConversationIdBytes,
    ) -> Result<Vec<String>> {
        let user = self.user.lock().await;
        Ok(user
            .group_members(conversation_id.into())
            .unwrap_or_default()
            .into_iter()
            .map(|c| c.to_string())
            .collect())
    }

    /// Get a list of contacts to be added to the conversation with the given
    /// [`ConversationId`].
    pub async fn member_candidates(
        &self,
        conversation_id: ConversationIdBytes,
    ) -> Result<Vec<UiContact>> {
        let user = self.user.lock().await;
        let group_members = user
            .group_members(conversation_id.into())
            .ok_or(anyhow!("Conversation not found"))?;
        let add_candidates = user
            .contacts()?
            .into_iter()
            .filter_map(|c| {
                if !group_members.contains(&c.user_name) {
                    Some(c.into())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        Ok(add_candidates)
    }
}
