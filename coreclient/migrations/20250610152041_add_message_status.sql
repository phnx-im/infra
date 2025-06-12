CREATE TABLE IF NOT EXISTS conversation_message_status (
    mimi_id BLOB NOT NULL,
    status INT NOT NULL,
    sender_user_domain TEXT NOT NULL,
    sender_user_uuid BLOB NOT NULL,
    PRIMARY KEY (
        mimi_id,
        status,
        sender_user_domain,
        sender_user_uuid
    )
);
