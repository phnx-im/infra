-- SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later

-- Create table to store any indexed keys
CREATE TABLE IF NOT EXISTS indexed_keys (
    key_index BLOB NOT NULL PRIMARY KEY,
    key_value BLOB NOT NULL,
    base_secret BLOB NOT NULL
);

-- Create table to store indices of own keys
CREATE TABLE IF NOT EXISTS own_key_indices (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    key_index BLOB NOT NULL,
    key_type TEXT CHECK(key_type IN ('user_profile_key')) NOT NULL,
    FOREIGN KEY (key_index) REFERENCES indexed_keys(key_index)
);

-- Recreate contacts table to add foreign key constraint
DROP TABLE IF EXISTS contacts;

CREATE TABLE IF NOT EXISTS contacts (
    user_name TEXT NOT NULL PRIMARY KEY,
    conversation_id BLOB NOT NULL,
    clients TEXT NOT NULL,
    wai_ear_key BLOB NOT NULL,
    friendship_token BLOB NOT NULL,
    key_package_ear_key BLOB NOT NULL,
    connection_key BLOB NOT NULL,
    user_profile_key_index BLOB NOT NULL,
    FOREIGN KEY (conversation_id) REFERENCES conversations (conversation_id),
    FOREIGN KEY (user_profile_key_index) REFERENCES indexed_keys (key_index)
);

CREATE TRIGGER IF NOT EXISTS delete_keys AFTER DELETE ON contacts FOR EACH ROW BEGIN
-- Delete user profile keys if the corresponding contact is deleted. Since key
-- indexes include the user name in their derivation, they are unique per user
-- and we don't need to check if they are used by another user (or ourselves).
DELETE FROM indexed_keys
WHERE
    fingerprint = OLD.user_profile_key_index;
END;
