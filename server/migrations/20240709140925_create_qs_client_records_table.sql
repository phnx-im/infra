-- SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later

-- migrations/{timestamp}_create_qs_client_records_table.sql
-- Create client records Table
CREATE TABLE qs_client_records(
client_id uuid NOT NULL,
PRIMARY KEY (client_id),
user_id uuid NOT NULL,
encrypted_push_token BYTEA,
owner_public_key BYTEA NOT NULL,
owner_signature_key BYTEA NOT NULL,
ratchet BYTEA NOT NULL,
activity_time timestamptz NOT NULL,
FOREIGN KEY (user_id) REFERENCES qs_user_records(user_id) ON DELETE CASCADE
);

