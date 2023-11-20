-- SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later

-- migrations/{timestamp}_create_queues_table.sql
-- Create Queues Table
CREATE TABLE queues(
message_id uuid NOT NULL,
PRIMARY KEY (message_id),
queue_id uuid NOT NULL,
sequence_number NUMERIC NOT NULL,
message_bytes BYTEA NOT NULL
);
