-- SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later

-- migrations/{timestamp}_create_queue_data_table.sql
-- Create QS Queue Data Table
CREATE TABLE qs_queue_data(
queue_id uuid NOT NULL,
PRIMARY KEY (queue_id),
sequence_number NUMERIC NOT NULL,
FOREIGN KEY (queue_id) REFERENCES qs_client_records(client_id) ON DELETE CASCADE
);
