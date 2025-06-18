# SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

set windows-shell := ["C:\\Program Files\\Git\\bin\\sh.exe","-c"]

# === Backend ===

POSTGRES_DATABASE_URL := "postgres://postgres:password@localhost:5432/phnx_db"

docker-is-podman := if `command -v podman || true` =~ ".*podman$" { "true" } else { "false" }

# run postgres via docker compose and apply migrations
init-db $DATABASE_URL=(POSTGRES_DATABASE_URL): generate-db-certs
    if {{docker-is-podman}} == "true"; then \
        podman-compose --podman-run-args=--replace up -d; \
    else \
        docker compose up --wait; \
    fi
    cd backend && sqlx database create
    cd backend && sqlx database setup

[working-directory: 'backend']
prepare-db-statements $DATABASE_URL=(POSTGRES_DATABASE_URL):
    cargo sqlx prepare --database-url $DATABASE_URL

# generate postgres TLS certificates
generate-db-certs:
    cd backend && TEST_CERT_DIR_NAME=test_certs scripts/generate_test_certs.sh

# === Client ===

[working-directory: 'coreclient']
init-client-db:
    sqlx database create --database-url sqlite://{{justfile_directory()}}/coreclient/client.db
    sqlx database setup --database-url sqlite://{{justfile_directory()}}/coreclient/client.db

[working-directory: 'coreclient']
prepare-client-db-statements: init-client-db
    cargo sqlx prepare --database-url sqlite://{{justfile_directory()}}/coreclient/client.db


# === App ===

app_lib_name := "applogic"
app_rust_base_dir := "../applogic"

# generate Dart files e.g. the data classes a.k.a freezed classes
[working-directory: 'app']
generate-dart-files:
    dart run build_runner build --delete-conflicting-outputs

# generate Rust and Dart flutter bridge files
[working-directory: 'app']
frb-generate $CARGO_TARGET_DIR=(justfile_directory() + "/target/frb_codegen"):
    rm -f {{app_rust_base_dir}}/src/frb_*.rs
    touch {{app_rust_base_dir}}/src/frb_generated.rs
    rm -Rf lib/core/api lib/core/frb_*.dart lib/core/lib.dart
    flutter_rust_bridge_codegen generate
    cd .. && cargo fmt

# Generate Rust and Dart flutter bridge files and check that they are committed
#
# Note: As a side effect, this recipe also checks whether the generated Dart
# files and the `app/pubspec.lock` file are up to date. This occurs because
# `flutter_rust_bridge_codegen` runs the `dart run build_runner build` command,
# which updates the generated files.
check-frb: frb-generate
    #!/usr/bin/env -S bash -eu
    if [ -n "$(git status --porcelain)" ]; then
        git add -N .
        git --no-pager diff
        echo -e "\x1b[1;31mFound uncommitted changes. Did you forget to run 'just frb-generate'?"
        exit 1
    fi

# same as check-generated-frb (with all prerequisite steps for running in CI)
check-frb-ci: install-cargo-binstall
    cargo binstall flutter_rust_bridge_codegen@2.10.0 cargo-expand
    just check-frb

# generate localization files
[working-directory: 'app']
gen-l10n:
    flutter gen-l10n

# check that the localization files are up to date
[working-directory: 'app']
check-l10n: gen-l10n
    #!/usr/bin/env -S bash -eu
    if [ -n "$(git status --porcelain)" ]; then
        git add -N .
        git --no-pager diff
        echo -e "\x1b[1;31mFound uncommitted changes. Did you forget to check in generated localization?"
        echo -e "\x1b[1;31mConsider to run 'just gen-l10n' manually."
        exit 1
    fi

# set up the CI environment for the app
install-cargo-binstall:
    curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash

# set up the CI environment for Android builds
[working-directory: 'app/fastlane']
setup-android-ci: install-cargo-binstall
    cargo binstall -y cargo-ndk
    bundle install

# set up the CI environment for iOS builds
[working-directory: 'app/fastlane']
setup-ios-ci: install-cargo-binstall
    bundle install

# set up the CI environment for macOS builds
[working-directory: 'app/fastlane']
setup-macos-ci: install-cargo-binstall
    bundle install

test-rust *args='':
    env DATABASE_URL={{POSTGRES_DATABASE_URL}} SQLX_OFFLINE=true cargo test {{args}}

# build Android
# we limit it to android-arm64 to speed up the build process
[working-directory: 'app']
build-android:
     flutter build appbundle --target-platform android-arm64

# build iOS
[working-directory: 'app']
build-ios:
    flutter build ios --no-codesign

# Build Linux app
[working-directory: 'app']
build-linux:
     flutter build linux

# analyze Dart code
[working-directory: 'app']
analyze-dart:
    cd rust_builder/cargokit/build_tool && flutter pub get
    flutter analyze

# run Flutter tests
[working-directory: 'app']
test-flutter *args='':
    flutter test {{args}}

# run backend server (at localhost)
run-backend: init-db
    cargo run --bin phnxserver

# Build Windows app
[working-directory: 'app']
build-windows:
     flutter build windows
