-- SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later

-- migrations/{timestamp}_create_opaque_setup_table.sql
-- Create opaque setup Table
CREATE TABLE opaque_setup(
id INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
opaque_setup BYTEA NOT NULL
);

