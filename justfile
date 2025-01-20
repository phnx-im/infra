# SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

export DATABASE_URL := "postgres://postgres:password@localhost:5432/phnx_db"

# === Backend ===

# run postgres via docker compose and apply migrations
init-db: generate-db-certs
    docker compose up --wait
    cd backend && sqlx database create
    cd backend && sqlx database setup

# generate postgres TLS certificates
generate-db-certs:
    cd backend && TEST_CERT_DIR_NAME=test_certs scripts/generate_test_certs.sh

# === Client ===

[working-directory: 'coreclient']
init-client-db:
    sqlx database create --database-url sqlite://client.db
    sqlx database setup --database-url sqlite://client.db

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
    rm -Rf lib/core
    mkdir lib/core
    flutter_rust_bridge_codegen generate

# integrate the Flutter Rust bridge
[working-directory: 'app']
frb-integrate:
    mv flutter_rust_bridge.yaml flutter_rust_bridge.yaml.tmp
    rm -Rf rust_builder test_driver
    flutter_rust_bridge_codegen integrate --rust-crate-name phnxapplogic --rust-crate-dir {{app_rust_base_dir}}
    git restore --source=HEAD --staged --worktree {{app_rust_base_dir}} lib
    git clean -fd {{app_rust_base_dir}} lib
    mv flutter_rust_bridge.yaml flutter_rust_bridge.yaml.generated.tmp
    echo "# This is only to inspect the generated flutter_rust_bridge.yaml file. Remove if not needed.\n" > /tmp/header.tmp
    cat /tmp/header.tmp flutter_rust_bridge.yaml.generated.tmp > flutter_rust_bridge.yaml.generated
    mv flutter_rust_bridge.yaml.tmp flutter_rust_bridge.yaml
    rm flutter_rust_bridge.yaml.generated.tmp
    just frb-generate

# set up the CI environment for the app
setup-ci:
    curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
    cargo binstall -y flutter_rust_bridge_codegen@2.7.0 cargo-expand

# set up the CI environment for Android builds
[working-directory: 'app/fastlane']
setup-android-ci: setup-ci
    cargo binstall -y cargo-ndk
    bundle install

# set up the CI environment for iOS builds
[working-directory: 'app/fastlane']
setup-ios-ci: setup-ci
	bundle install

# build Android
# we limit it to android-arm64 to speed up the build process
[working-directory: 'app']
build-android:
     flutter build appbundle --target-platform android-arm64

# build iOS
[working-directory: 'app']
build-ios:
	flutter build ios --no-codesign

# analyze Dart code
[working-directory: 'app']
analyze-dart:
    flutter analyze

# run Flutter tests
[working-directory: 'app']
test-flutter:
    flutter test
