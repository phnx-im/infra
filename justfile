# SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

export DATABASE_URL := "postgres://postgres:password@localhost:5432/phnx_db"

set windows-shell := ["C:\\Program Files\\Git\\bin\\sh.exe","-c"]

# === Backend ===

# run postgres via docker compose and apply migrations
init-db: generate-db-certs
    docker compose up --wait
    cd backend && sqlx database create
    cd backend && sqlx database setup

# generate postgres TLS certificates
generate-db-certs:
    cd backend && TEST_CERT_DIR_NAME=test_certs scripts/generate_test_certs.sh

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

# compare the generated files with the current files
frb-compare $CARGO_TARGET_DIR=(justfile_directory() + "/target/frb_codegen"):
    rm -Rf /tmp/frb-temp-files
    cp -R . /tmp/frb-temp-files
    (cd /tmp/frb-temp-files/app && flutter_rust_bridge_codegen generate --dart-output /tmp/frb-temp-files/app/lib/core)
    (cd /tmp/frb-temp-files/app && dart run build_runner build --delete-conflicting-outputs)
    diff -r /tmp/frb-temp-files/app/lib/core app/lib/core

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
    cargo binstall -y flutter_rust_bridge_codegen@2.7.1 cargo-expand

# set up the CI environment for Android builds
[working-directory: 'app/fastlane']
setup-android-ci: setup-ci
    cargo binstall -y cargo-ndk
    bundle install

# set up the CI environment for iOS builds
[working-directory: 'app/fastlane']
setup-ios-ci: setup-ci
	bundle install

# set up the CI environment for macOS builds
[working-directory: 'app/fastlane']
setup-macos-ci: setup-ci
	bundle install

test-rust *args='':
    cargo test {{args}}

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

# Build Linux app (with all prerequisite steps for running in CI)
[working-directory: 'app']
build-linux-ci: setup-ci build-linux

# analyze Dart code
[working-directory: 'app']
analyze-dart:
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

# Build Windows app (with all prerequisite steps for running in CI)
[working-directory: 'app']
build-windows-ci: setup-ci build-windows
