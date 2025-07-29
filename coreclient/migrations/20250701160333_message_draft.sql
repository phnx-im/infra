-- SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later
--
-- Adds support for message drafts per conversation.
--
CREATE TABLE IF NOT EXISTS conversation_message_draft (
    conversation_id BLOB NOT NULL PRIMARY KEY,
    message TEXT NOT NULL,
    editing_id BLOB,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (conversation_id) REFERENCES conversations (conversation_id) ON DELETE CASCADE,
    FOREIGN KEY (editing_id) REFERENCES conversation_messages (message_id) ON DELETE CASCADE
);
