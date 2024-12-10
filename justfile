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

# === App ===

app_dir := "prototype"
app_lib_name := "applogic"
app_rust_base_dir := app_lib_name

# generate Rust and Dart flutter bridge files
frb-generate:
    rm -f {{app_rust_base_dir}}/src/frb_*.rs
    touch {{app_rust_base_dir}}/src/frb_generated.rs
    mkdir -p {{app_dir}}/lib/core
    rm -Rf {{app_dir}}/lib/core/*
    cd {{app_dir}} && flutter pub get
    cd {{app_dir}} && flutter_rust_bridge_codegen generate
    cd {{app_dir}} && flutter clean

# integrate the Flutter Rust bridge
frb-integrate:
	cd {{app_dir}} && mv flutter_rust_bridge.yaml flutter_rust_bridge.yaml.tmp
	cd {{app_dir}} && rm -Rf rust_builder test_driver
	cd {{app_dir}} && flutter_rust_bridge_codegen integrate --rust-crate-name phnxapplogic --rust-crate-dir ../{{app_rust_base_dir}}
	cd {{app_dir}} && git restore --source=HEAD --staged --worktree ../{{app_rust_base_dir}} lib
	cd {{app_dir}} && git clean -fd ../{{app_rust_base_dir}} lib
	cd {{app_dir}} && mv flutter_rust_bridge.yaml flutter_rust_bridge.yaml.generated.tmp
	cd {{app_dir}} && echo "# This is only to inspect the generated flutter_rust_bridge.yaml file. Remove if not needed.\n" > /tmp/header.tmp
	cd {{app_dir}} && cat /tmp/header.tmp flutter_rust_bridge.yaml.generated.tmp > flutter_rust_bridge.yaml.generated
	cd {{app_dir}} && mv flutter_rust_bridge.yaml.tmp flutter_rust_bridge.yaml
	cd {{app_dir}} && rm flutter_rust_bridge.yaml.generated.tmp
	just frb-generate

# set up the CI environment for the app
setup-ci:
	curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
	cargo binstall -y flutter_rust_bridge_codegen@2.6.0 cargo-expand

# set up the CI environment for Android builds
setup-android-ci: setup-ci
	cargo binstall -y cargo-ndk
	cd {{app_dir}}/fastlane && bundle install

# set up the CI environment for iOS builds
setup-ios-ci: setup-ci
	cd {{app_dir}}/fastlane && bundle install

# build Android
# we limit it to android-arm64 to speed up the build process
build-android:
     cd {{app_dir}} && flutter build appbundle --target-platform android-arm64

# build iOS
build-ios:
	cd {{app_dir}} && flutter build ios --no-codesign
