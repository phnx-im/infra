-- SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later
CREATE TABLE IF NOT EXISTS as_user_handles_queues (
    message_id uuid PRIMARY KEY,
    hash BYTEA NOT NULL,
    message_bytes BYTEA NOT NULL,
    fetched_by uuid,
    created_at timestamptz DEFAULT now (),
    FOREIGN KEY (hash) REFERENCES as_user_handles (hash) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS as_user_handles_queues_created_at ON as_user_handles_queues (created_at);

CREATE INDEX IF NOT EXISTS as_user_handles_queues_fetched_by ON as_user_handles_queues (hash, fetched_by)
WHERE
    fetched_by IS NOT NULL;
