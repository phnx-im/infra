# SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

export DATABASE_URL := "postgres://postgres:password@localhost:5432/phnx_db"

# run postgres via docker compose and apply migrations
init-db: generate-db-certs
    docker compose up --wait
    cd backend && sqlx database create
    cd backend && sqlx database setup

# generate postgres TLS certificates
generate-db-certs:
    cd backend && TEST_CERT_DIR_NAME=test_certs scripts/generate_test_certs.sh

# generate Rust and Dart flutter bridge files
generate-flutter-ffi:
    cd prototype && make frb-generate
