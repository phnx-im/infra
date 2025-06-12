-- SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later
CREATE TABLE IF NOT EXISTS as_queue_data (
    queue_id uuid PRIMARY KEY,
    sequence_number BIGINT NOT NULL,
    FOREIGN KEY (queue_id) REFERENCES as_client_records (user_uuid) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS as_queues (
    message_id uuid NOT NULL,
    queue_id uuid NOT NULL,
    sequence_number BIGINT NOT NULL,
    message_bytes BYTEA NOT NULL,
    PRIMARY KEY (queue_id, sequence_number),
    FOREIGN KEY (queue_id) REFERENCES as_queue_data (queue_id) ON DELETE CASCADE
);

ALTER TABLE as_client_records
ADD COLUMN queue_encryption_key BYTEA NOT NULL DEFAULT '\x00';

ALTER TABLE as_client_records
ADD COLUMN ratchet BYTEA NOT NULL DEFAULT '\x00';

CREATE TABLE connection_packages (
    id INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_uuid uuid NOT NULL,
    connection_package BYTEA NOT NULL,
    FOREIGN KEY (user_uuid) REFERENCES as_client_records (user_uuid) ON DELETE CASCADE
);

CREATE INDEX idx_connection_package_user_uuid ON connection_packages (user_uuid);
