-- SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later
--
-- All attachments
CREATE TABLE IF NOT EXISTS attachments (
    attachment_id BLOB NOT NULL PRIMARY KEY,
    conversation_id BLOB NOT NULL,
    conversation_message_id BLOB NOT NULL,
    content_type TEXT NOT NULL,
    content BLOB,
    status INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (conversation_id) REFERENCES conversations (conversation_id) ON DELETE CASCADE,
    FOREIGN KEY (conversation_message_id) REFERENCES conversation_messages (message_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS attachment_created_at_index ON attachments (created_at);

-- Additional data needed for downloading attachments.
--
-- After the attachment is downloaded, the record can be deleted.
CREATE TABLE IF NOT EXISTS pending_attachments (
    attachment_id BLOB NOT NULL PRIMARY KEY,
    size INTEGER NOT NULL,
    enc_alg INTEGER NOT NULL,
    enc_key BLOB NOT NULL,
    nonce BLOB NOT NULL,
    aad BLOB NOT NULL,
    hash_alg INTEGER NOT NULL,
    hash BLOB NOT NULL,
    FOREIGN KEY (attachment_id) REFERENCES attachments (attachment_id) ON DELETE CASCADE
);
