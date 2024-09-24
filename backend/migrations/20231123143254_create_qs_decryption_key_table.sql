-- SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later

-- migrations/{timestamp}_create_qs_decryption_key_table.sql
-- Create decryption key Table
CREATE TABLE qs_decryption_key(
id uuid NOT NULL,
PRIMARY KEY (id),
decryption_key BYTEA NOT NULL
);
