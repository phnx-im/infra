implement_error! {
    pub enum ConversationStoreError{
        Simple {
            UnknownConversation = "The conversation does not exist",
            MissingMessageId = "The message is missing an ID",
        }
        Complex {}
    }
}
