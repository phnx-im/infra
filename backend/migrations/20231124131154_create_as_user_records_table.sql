-- SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later

-- migrations/{timestamp}_create_as_user_records_table.sql
-- Create user records Table
CREATE TABLE as_user_records(
user_name TEXT UNIQUE NOT NULL,
PRIMARY KEY (user_name),
password_file BYTEA NOT NULL
);