-- SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later

-- migrations/{timestamp}_create_groups_table.sql
-- Create Groups Table
CREATE TABLE encrypted_groups(
group_id uuid NOT NULL,
PRIMARY KEY (group_id),
ciphertext BYTEA NOT NULL,
last_used timestamptz NOT NULL,
deleted_queues BYTEA NOT NULL
);