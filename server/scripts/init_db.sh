#!/usr/bin/env bash

# SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

set -x
set -eo pipefail
if ! [ -x "$(command -v psql)" ]; then
    echo >&2 "Error: psql is not installed."
    exit 1
fi

if ! [ -x "$(command -v sqlx)" ]; then
    echo >&2 "Error: sqlx is not installed."
    echo >&2 "Use:"
    echo >&2 "    cargo install --version=0.5.7 sqlx-cli --no-default-features --features postgres"
    echo >&2 "to install it."
    exit 1
fi

DB_USER=${POSTGRES_USER:=postgres}
DB_PASSWORD="${POSTGRES_PASSWORD:=password}"
DB_NAME="${POSTGRES_DB:=ds_db}"
DB_PORT="${POSTGRES_PORT:=5432}"

# Name of the directory for the test certs
TEST_CERT_DIR_NAME="./test_certs"

# Check if directory exists
if [ -d "$TEST_CERT_DIR_NAME" ]; then
  echo "Directory $TEST_CERT_DIR_NAME already exists. Skipping certificate generation."
else
  echo "Directory $TEST_CERT_DIR_NAME does not exist. Creating directory and generating certificates."
  
  # Create directory
  mkdir -p "$TEST_CERT_DIR_NAME"
  
  # Generate CA private key and self-signed certificate
  openssl req -new -x509 -days 36500 -nodes -out "$TEST_CERT_DIR_NAME/root.crt" -keyout "$TEST_CERT_DIR_NAME/root.key" -subj "/CN=Test Root CA"
  
  # Generate server private key and certificate signing request (CSR)
  openssl req -new -nodes -out "$TEST_CERT_DIR_NAME/server.csr" -keyout "$TEST_CERT_DIR_NAME/server.key" -subj "/CN=test.postgres.server"
  
  # Sign the server certificate with the CA certificate
  openssl x509 -req -in "$TEST_CERT_DIR_NAME/server.csr" -CA "$TEST_CERT_DIR_NAME/root.crt" -CAkey "$TEST_CERT_DIR_NAME/root.key" -CAcreateserial -out "$TEST_CERT_DIR_NAME/server.crt" -days 36500
  
  # Set permissions for the server key
  chmod 600 "$TEST_CERT_DIR_NAME/server.key"
  
  echo "Certificates and configuration file generated in $TEST_CERT_DIR_NAME."
fi

if [[ -z "${SKIP_DOCKER}" ]]
then
    docker run \
        -v $TEST_CERT_DIR_NAME:/etc/postgres_certs:ro \
        -e POSTGRES_USER=${DB_USER} \
        -e POSTGRES_PASSWORD=${DB_PASSWORD} \
        -e POSTGRES_DB=${DB_NAME} \
        -p "${DB_PORT}":5432 \
        -d postgres \
        -N 1000 \
        -c ssl=on \
        -c ssl_cert_file=/etc/postgres_certs/server.crt \
        -c ssl_key_file=/etc/postgres_certs/server.key \
        -c ssl_ca_file=/etc/postgres_certs/root.crt
fi

# Keep pinging Postgres until it's ready to accept commands
export PGPASSWORD="${DB_PASSWORD}"
export PGSSLMODE="verify-ca"
export PGSSLROOTCERT="$TEST_CERT_DIR_NAME/root.crt"
until psql -h "localhost" -U "${DB_USER}" -p "${DB_PORT}" -d "postgres" -c '\q'; do
    >&2 echo "Postgres is still unavailable - sleeping"
    sleep 1
done

>&2 echo "Postgres is up and running on port ${DB_PORT}!"

export DATABASE_URL=postgres://${DB_USER}:${DB_PASSWORD}@localhost:${DB_PORT}/${DB_NAME}
sqlx database create

sqlx migrate run
>&2 echo "Postgres has been migrated, ready to go!"
