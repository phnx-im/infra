-- SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later
--
-- Introduces a tables storing the history of edits to a message.
CREATE TABLE IF NOT EXISTS message_edit (
    -- This is the Mimi ID of the `content` field.
    mimi_id BLOB NOT NULL PRIMARY KEY,
    -- the message that was edited
    --
    -- The content of the message is always the latest version of the message.
    -- That is, the latest edit contains the previous message content. The
    -- second latest edit contains the content before the latest edit, and so
    -- on.
    message_id BLOB NOT NULL,
    created_at TEXT NOT NULL,
    -- content of the edited message
    content BLOB NOT NULL,
    FOREIGN KEY (message_id) REFERENCES conversation_messages (message_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS message_edit_message_id_idx ON message_edit (message_id);

ALTER TABLE conversation_messages
ADD COLUMN edited_at TEXT;
