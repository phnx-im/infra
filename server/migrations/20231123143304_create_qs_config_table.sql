-- SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later

-- migrations/{timestamp}_create_qs_config_table.sql
-- Create config Table
CREATE TABLE qs_config(
id uuid NOT NULL,
PRIMARY KEY (id),
config BYTEA NOT NULL
);

