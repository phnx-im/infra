#!/usr/bin/env bash

# SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

set -euo pipefail

echo "Test directory: $TEST_CERT_DIR_NAME"

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

  # Create a config file for X.509v3 extensions (this is necessary, because the
  # openssl version on the CI doesn't generate the correct extensions by
  # default)
#  cat > "$TEST_CERT_DIR_NAME/server.cnf" <<EOF
#[req]
#distinguished_name = req_distinguished_name
#[req_distinguished_name]
#CN = test.postgres.server
#
#[v3_req]
#basicConstraints = CA:FALSE
#keyUsage = digitalSignature, keyEncipherment
#extendedKeyUsage = serverAuth
#subjectKeyIdentifier = hash
#authorityKeyIdentifier = keyid,issuer
#EOF
  
  # Sign the server certificate with the CA certificate
  openssl x509 -req -days 36500 \
    -in "$TEST_CERT_DIR_NAME/server.csr" \
    -CA "$TEST_CERT_DIR_NAME/root.crt" \
    -CAkey "$TEST_CERT_DIR_NAME/root.key" \
    -CAcreateserial \
    -out "$TEST_CERT_DIR_NAME/server.crt" \
    #-extfile "$TEST_CERT_DIR_NAME/server.cnf" \
    #-extensions v3_req

  # Set permissions for the server key
  chmod 600 "$TEST_CERT_DIR_NAME/server.key"
  
  echo "Certificates and configuration file generated in $TEST_CERT_DIR_NAME."

  # Check and change ownership if we're running on the CI
  if [[ "${CI:-}" == "true" ]]; then
    echo "Running on CI, changing ownership of files in $TEST_CERT_DIR_NAME to user with UID 999, which corresponds to the postgres user in the postgres docker container."
    sudo chown -R 999:999 "$TEST_CERT_DIR_NAME/server.crt" "$TEST_CERT_DIR_NAME/server.key"
  else
    echo "Not running on CI. Skipping ownership change."
  fi
fi

echo "Server Certificate Details:"
openssl x509 -in "$TEST_CERT_DIR_NAME/server.crt" -text -noout | cat


