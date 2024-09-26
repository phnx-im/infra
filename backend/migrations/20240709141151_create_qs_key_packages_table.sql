-- SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later

-- migrations/{timestamp}_create_key_packages_table.sql
-- Create KeyPackages Table
CREATE TABLE key_packages(
id INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
client_id uuid NOT NULL,
encrypted_add_package BYTEA NOT NULL,
is_last_resort BOOLEAN NOT NULL,
FOREIGN KEY (client_id) REFERENCES qs_client_records(client_id) ON DELETE CASCADE
);

CREATE INDEX idx_key_package_client_id ON key_packages(client_id);
