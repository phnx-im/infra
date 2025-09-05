-- SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later
--
-- Add ON DELETE CASCADE to the foreign key constraints referencing the
-- conversations table.
--
PRAGMA foreign_keys = OFF;

-- Don't update foreign references when renaming tables
PRAGMA legacy_alter_table = ON;

DROP TRIGGER IF EXISTS delete_keys;

-- Migration for 'contacts' table
ALTER TABLE contacts
RENAME TO contacts_old;

CREATE TABLE IF NOT EXISTS contacts (
    user_uuid BLOB NOT NULL,
    user_domain TEXT NOT NULL,
    conversation_id BLOB NOT NULL REFERENCES conversations (conversation_id) ON DELETE CASCADE,
    wai_ear_key BLOB NOT NULL,
    friendship_token BLOB NOT NULL,
    connection_key BLOB NOT NULL,
    user_profile_key_index BLOB NOT NULL REFERENCES indexed_keys (key_index),
    PRIMARY KEY (user_uuid, user_domain)
);

INSERT INTO
    contacts
SELECT
    co.*
FROM
    contacts_old co
    INNER JOIN conversations conv ON co.conversation_id = conv.conversation_id;

DROP TABLE contacts_old;

-- Migration for 'conversation_messages' table
ALTER TABLE conversation_messages
RENAME TO conversation_messages_old;

CREATE TABLE conversation_messages (
    message_id BLOB NOT NULL PRIMARY KEY,
    conversation_id BLOB NOT NULL REFERENCES conversations (conversation_id) ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED,
    timestamp TEXT NOT NULL,
    -- missing `sender_as_{client_uuid,domain}` fields means it is a system message
    sender_user_uuid BLOB,
    sender_user_domain TEXT,
    content BLOB NOT NULL,
    sent BOOLEAN NOT NULL,
    mimi_id BLOB,
    status INT NOT NULL DEFAULT 0,
    edited_at TEXT
);

INSERT INTO
    conversation_messages
SELECT
    *
FROM
    conversation_messages_old;

DROP TABLE conversation_messages_old;

CREATE TRIGGER IF NOT EXISTS delete_keys AFTER DELETE ON contacts FOR EACH ROW BEGIN
-- Delete user profile keys if the corresponding contact is deleted. Since key
-- indexes include the user id in their derivation, they are unique per user
-- and we don't need to check if they are used by another user (or ourselves).
DELETE FROM indexed_keys
WHERE
    key_index = OLD.user_profile_key_index;

END;

PRAGMA foreign_keys = ON;

PRAGMA legacy_alter_table = OFF;
