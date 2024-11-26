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
  
  # Sign the server certificate with the CA certificate
  openssl x509 -req -in "$TEST_CERT_DIR_NAME/server.csr" -CA "$TEST_CERT_DIR_NAME/root.crt" -CAkey "$TEST_CERT_DIR_NAME/root.key" -CAcreateserial -out "$TEST_CERT_DIR_NAME/server.crt" -days 36500

  # Set permissions for the server key
  chmod 600 "$TEST_CERT_DIR_NAME/server.key"
  
  echo "Certificates and configuration file generated in $TEST_CERT_DIR_NAME."
fi