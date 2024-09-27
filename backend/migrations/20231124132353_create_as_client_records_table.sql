-- SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later

-- migrations/{timestamp}_create_as_client_records_table.sql
-- Create client records Table
CREATE TABLE as_client_records(
client_id uuid NOT NULL,
user_name TEXT NOT NULL,
queue_encryption_key BYTEA NOT NULL,
ratchet BYTEA NOT NULL,
activity_time timestamptz NOT NULL,
client_credential BYTEA NOT NULL,
remaining_tokens integer NOT NULL,
PRIMARY KEY (client_id),
FOREIGN KEY (user_name) REFERENCES as_user_records(user_name) ON DELETE CASCADE
);

