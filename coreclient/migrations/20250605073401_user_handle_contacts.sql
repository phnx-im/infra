-- SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later
--
-- Adds a new tables for storing partial contacts based on a user handle.
-- Add a new column to the conversations table to store the user handle.
--
CREATE TABLE IF NOT EXISTS user_handle_contacts (
    user_handle TEXT NOT NULL PRIMARY KEY,
    -- 1:1 relationship with conversations
    conversation_id BLOB NOT NULL UNIQUE,
    friendship_package_ear_key BLOB NOT NULL,
    created_at TEXT NOT NULL,
    connection_offer_hash BLOB NOT NULL,
    FOREIGN KEY (conversation_id) REFERENCES conversations (conversation_id) ON DELETE CASCADE
);

ALTER TABLE conversations
ADD COLUMN connection_user_handle TEXT;
