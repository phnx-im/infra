-- SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later

-- migrations/{timestamp}_create_connection_packages_table.sql
-- Create ConnectionPackages Table
CREATE TABLE connection_packages(
id uuid NOT NULL,
PRIMARY KEY (id),
client_id uuid NOT NULL,
connection_package BYTEA NOT NULL,
FOREIGN KEY (client_id) REFERENCES as_client_records(client_id) ON DELETE CASCADE
);

CREATE INDEX idx_connection_package_client_id ON connection_packages(client_id);