-- SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later

-- migrations/{timestamp}_create_qs_decryption_key_table.sql
-- Create decryption key Table

CREATE TYPE decryption_key_data AS (
    encryption_key BYTEA,
    decryption_key BYTEA
);

CREATE TABLE qs_decryption_key(
id INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
decryption_key decryption_key_data NOT NULL
);
