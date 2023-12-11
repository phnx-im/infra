-- SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later

-- migrations/{timestamp}_create_qs_user_records_table.sql
-- Create user records Table
CREATE TABLE qs_user_records(
user_id uuid NOT NULL,
PRIMARY KEY (user_id),
friendship_token BYTEA NOT NULL,
verifying_key BYTEA NOT NULL
);
