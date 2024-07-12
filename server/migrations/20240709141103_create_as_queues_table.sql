-- SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later

-- migrations/{timestamp}_create_queues_table.sql
-- Create AS Queues Table
CREATE TABLE as_queues(
message_id uuid NOT NULL,
queue_id uuid NOT NULL,
sequence_number NUMERIC NOT NULL,
message_bytes BYTEA NOT NULL,
PRIMARY KEY (queue_id, sequence_number),
FOREIGN KEY (queue_id) REFERENCES as_queue_data(queue_id) ON DELETE CASCADE
);
