-- SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later

-- migrations/{timestamp}_create_queue_data_table.sql
-- Create AS Queue Data Table
CREATE TABLE as_queue_data(
queue_id uuid NOT NULL,
PRIMARY KEY (queue_id),
sequence_number BIGINT NOT NULL,
FOREIGN KEY (queue_id) REFERENCES as_client_records(client_id) ON DELETE CASCADE
);
