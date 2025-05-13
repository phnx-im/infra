-- SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later

-- Recreate users table to add decryption key index
DROP TABLE IF EXISTS users;

CREATE TABLE IF NOT EXISTS users (
    user_name TEXT NOT NULL PRIMARY KEY,
    epoch INTEGER NOT NULL,
    decryption_key_index BLOB NOT NULL,
    display_name TEXT NOT NULL,
    profile_picture BLOB
);