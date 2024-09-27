-- SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later

-- migrations/{timestamp}_create_as_batched_keys_table.sql
CREATE TABLE as_batched_keys(
token_key_id smallint NOT NULL,
PRIMARY KEY (token_key_id),
voprf_server BYTEA NOT NULL
);
