-- SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later
--
-- Support for attachments
CREATE TABLE IF NOT EXISTS attachments (
    attachment_id BLOB NOT NULL PRIMARY KEY,
    conversation_id BLOB NOT NULL,
    content_type TEXT NOT NULL,
    content BLOB,
    status INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (conversation_id) REFERENCES conversations (conversation_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS attachment_created_at_index ON attachments (created_at);

CREATE TABLE IF NOT EXISTS attachment_images (
    attachment_id BLOB NOT NULL PRIMARY KEY,
    blurhash TEXT NOT NULL,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    FOREIGN KEY (attachment_id) REFERENCES attachments (attachment_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS attachment_thumbnails (
    attachment_id BLOB NOT NULL PRIMARY KEY,
    content_type TEXT NOT NULL,
    content BLOB NOT NULL,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    FOREIGN KEY (attachment_id) REFERENCES attachments (attachment_id) ON DELETE CASCADE
);
