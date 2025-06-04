-- SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later
--
-- 1. Add ON DELETE CASCADE to the foreign key constraints referencing the
-- conversations table.
-- 2. Fix invalid TRIGGER reference to indexed_keys table (replaced fingerprint
-- with key_index).
--
PRAGMA foreign_keys = ON;

DROP TRIGGER IF EXISTS no_partial_contact_overlap_on_insert;

DROP TRIGGER IF EXISTS no_partial_contact_overlap_on_update;

DROP TRIGGER IF EXISTS no_contact_overlap_on_insert;

DROP TRIGGER IF EXISTS no_contact_overlap_on_update;

DROP TRIGGER IF EXISTS delete_keys;

-- Migration for 'contacts' table
ALTER TABLE contacts
RENAME TO contacts_old;

CREATE TABLE IF NOT EXISTS contacts (
    user_uuid BLOB NOT NULL,
    user_domain TEXT NOT NULL,
    conversation_id BLOB NOT NULL,
    wai_ear_key BLOB NOT NULL,
    friendship_token BLOB NOT NULL,
    connection_key BLOB NOT NULL,
    user_profile_key_index BLOB NOT NULL,
    PRIMARY KEY (user_uuid, user_domain),
    FOREIGN KEY (conversation_id) REFERENCES conversations (conversation_id) ON DELETE CASCADE,
    FOREIGN KEY (user_profile_key_index) REFERENCES indexed_keys (key_index)
);

INSERT INTO
    contacts
SELECT
    co.*
FROM
    contacts_old co
    INNER JOIN conversations conv ON co.conversation_id = conv.conversation_id;

DROP TABLE contacts_old;

-- Migration for 'partial_contacts' table
ALTER TABLE partial_contacts
RENAME TO partial_contacts_old;

CREATE TABLE IF NOT EXISTS partial_contacts (
    user_uuid BLOB NOT NULL,
    user_domain TEXT NOT NULL,
    conversation_id BLOB NOT NULL,
    friendship_package_ear_key BLOB NOT NULL,
    PRIMARY KEY (user_uuid, user_domain),
    FOREIGN KEY (conversation_id) REFERENCES conversations (conversation_id) ON DELETE CASCADE
);

INSERT INTO
    partial_contacts
SELECT
    pc.*
FROM
    partial_contacts_old pc
    INNER JOIN conversations conv ON pc.conversation_id = conv.conversation_id;

DROP TABLE partial_contacts_old;

-- Migration for 'conversation_messages' table
ALTER TABLE conversation_messages
RENAME TO conversation_messages_old;

CREATE TABLE IF NOT EXISTS conversation_messages (
    message_id BLOB NOT NULL PRIMARY KEY,
    conversation_id BLOB NOT NULL,
    timestamp TEXT NOT NULL,
    sender_user_uuid BLOB,
    sender_user_domain TEXT,
    content BLOB NOT NULL,
    sent BOOLEAN NOT NULL,
    FOREIGN KEY (conversation_id) REFERENCES conversations (conversation_id) ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED
);

INSERT INTO
    conversation_messages
SELECT
    *
FROM
    conversation_messages_old;

DROP TABLE conversation_messages_old;

CREATE TRIGGER IF NOT EXISTS no_partial_contact_overlap_on_insert BEFORE INSERT ON contacts FOR EACH ROW BEGIN
SELECT
    CASE
        WHEN EXISTS (
            SELECT
                1
            FROM
                partial_contacts
            WHERE
                user_uuid = NEW.user_uuid
                AND user_domain = NEW.user_domain
        ) THEN RAISE (
            FAIL,
            'Can''t insert Contact: There already exists a partial contact with this client_id and domain'
        )
    END;

END;

CREATE TRIGGER IF NOT EXISTS no_partial_contact_overlap_on_update BEFORE
UPDATE ON contacts FOR EACH ROW BEGIN
SELECT
    CASE
        WHEN EXISTS (
            SELECT
                1
            FROM
                partial_contacts
            WHERE
                user_uuid = NEW.user_uuid
                AND user_domain = NEW.user_domain
        ) THEN RAISE (
            FAIL,
            'Can''t update Contact: There already exists a partial contact with this client_id and domain'
        )
    END;

END;

CREATE TRIGGER IF NOT EXISTS no_contact_overlap_on_insert BEFORE INSERT ON partial_contacts FOR EACH ROW BEGIN
SELECT
    CASE
        WHEN EXISTS (
            SELECT
                1
            FROM
                contacts
            WHERE
                user_uuid = NEW.user_uuid
                AND user_domain = NEW.user_domain
        ) THEN RAISE (
            FAIL,
            'Can''t insert PartialContact: There already exists a contact with this client_id and domain'
        )
    END;

END;

CREATE TRIGGER IF NOT EXISTS no_contact_overlap_on_update BEFORE
UPDATE ON partial_contacts FOR EACH ROW BEGIN
SELECT
    CASE
        WHEN EXISTS (
            SELECT
                1
            FROM
                contacts
            WHERE
                user_uuid = NEW.user_uuid
                AND user_domain = NEW.user_domain
        ) THEN RAISE (
            FAIL,
            'Can''t update PartialContact: There already exists a contact with this client_id and domain'
        )
    END;

END;

CREATE TRIGGER IF NOT EXISTS delete_keys AFTER DELETE ON contacts FOR EACH ROW BEGIN
-- Delete user profile keys if the corresponding contact is deleted. Since key
-- indexes include the user id in their derivation, they are unique per user
-- and we don't need to check if they are used by another user (or ourselves).
DELETE FROM indexed_keys
WHERE
    key_index = OLD.user_profile_key_index;

END;
