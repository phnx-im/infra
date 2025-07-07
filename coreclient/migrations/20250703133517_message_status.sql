-- SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later
--
-- Add mimi id to conversation messages and a table for storing message statuses per message and user.
--
-- The combination of message id, user id and status is unique.
--
CREATE TABLE IF NOT EXISTS conversation_message_status (
    message_id BLOB NOT NULL,
    sender_user_uuid BLOB NOT NULL,
    sender_user_domain TEXT NOT NULL,
    -- Bit set of message statuses in the form `Sum_status 2**status`
    status_bitset INT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (message_id, sender_user_domain, sender_user_uuid),
    FOREIGN KEY (message_id) REFERENCES conversation_messages (message_id) ON DELETE CASCADE
);

ALTER TABLE conversation_messages
ADD COLUMN mimi_id BLOB;

ALTER TABLE conversation_messages
-- Aggregated status bitset from the conversation_message_status table.
--
-- The set contains all statuses that at least one user has set for this
-- message, that is, the aggregation is done by ORing the status bits.
--
-- Technically, this field can be computed from the
-- `conversation_message_status` table, however, sqlite does not support BIT_OR
-- aggregate functions, so we store the aggregated value here.
ADD COLUMN status_bitset INT NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS conversation_message_mimi_id_idx ON conversation_messages (mimi_id);
