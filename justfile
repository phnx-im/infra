# SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

export RUST_LOG := "info"
export RUST_BACKTRACE := "1"
export SQLX_OFFLINE := "true"
export DATABASE_URL := "postgres://postgres:password@localhost:5432/phnx_db"
export RUSTFLAGS := "-D warnings"

_default:
    just --list

# Reset and migrate databases.
reset-dev:
    cd coreclient && cargo sqlx database reset -y
    cd backend && cargo sqlx database reset -y

@check-frb:
    just _check-unstaged-changes "just regenerate-frb"

@check-rust:
    just _check-status "cargo machete"
    just _check-status "reuse lint -l"
    just _check-status "cargo metadata --format-version 1 --locked > /dev/null"
    just _check-status "cargo fmt -- --check"
    just _check-status "cargo deny check"
    echo "{{BOLD}}check-rust done{{NORMAL}}"

@check-flutter:
    just _check-status "git lfs --version"
    just _check-unstaged-changes "git diff"
    just _check-unstaged-changes "cd app && fvm flutter pub get"
    just _check-unstaged-changes "cd app/rust_builder/cargokit/build_tool && fvm flutter pub get"
    just _check-unstaged-changes "cd app && fvm dart format ."
    just _check-status "cd app && fvm flutter analyze --no-pub"
    echo "{{BOLD}}check-flutter done{{NORMAL}}"

# Run many fast and simple lints.
@check: check-rust check-flutter

_check-status command:
    #!/usr/bin/env -S bash -eu
    echo "{{BOLD}}Running {{command}}{{NORMAL}}"
    if ! log=$({{command}} 2>&1); then
        echo "{{RED}}$log{{NORMAL}}" >&2
        just _log-error "{{command}}"
    fi

_check-unstaged-changes command:
    #!/usr/bin/env -S bash -eu
    echo "{{BOLD}}Running {{command}}{{NORMAL}}"
    {{command}} >/dev/null
    if ! git diff --quiet; then
        echo -e "{{RED}}Found unstaged changes.{{NORMAL}}"
        just _log-error "{{command}}"
    fi

_log-error msg:
    #!/usr/bin/env -S bash -eu
    if [[ -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
        echo -e "::error::{{msg}}"
    else
        msg="\x1b[1;31mERROR: {{msg}}\x1b[0m"
        echo -e "$msg"
    fi
    exit 1


# Regenerate Flutter-Rust bridge files and l10n files.
regenerate-glue: regenerate-frb regenerate-l10n

[working-directory: 'app']
regenerate-frb:
    rm -f ../applogic/src/frb_*.rs
    touch ../applogic/src/frb_generated.rs
    rm -Rf lib/core/api lib/core/frb_*.dart lib/core/lib.dart

    CARGO_TARGET_DIR="{{justfile_directory()}}/target/frb_codegen" \
        flutter_rust_bridge_codegen generate

    cd .. && cargo fmt

regenerate-l10n:
    fvm flutter gen-l10n


# Run cargo build, clippy and test.
@test-rust:
    just _check-status "cargo clippy --locked --all-targets"
    just _check-status "just run-docker-compose && cargo test --locked -q"
    echo "{{BOLD}}test-rust done{{NORMAL}}"

# Run flutter test.
test-flutter:
    cd app && fvm flutter test
    echo "{{BOLD}}test-flutter done{{NORMAL}}"

ci: check test

# Run all tests.
test: test-rust test-flutter

docker-is-podman := if `command -v podman || true` =~ ".*podman$" { "true" } else { "false" }
# Run docker compose services in the background.
@run-docker-compose: _generate-db-certs
    if {{docker-is-podman}} == "true"; then \
        podman rm infra_minio-setup_1 -i 2>&1 /dev/null; \
        podman-compose --podman-run-args=--replace up -d; \
        podman-compose ps; \
        podman logs infra_postgres_1; \
    else \
        docker compose up --wait --wait-timeout=300; \
        docker compose ps; \
    fi

# Generate postgres TLS certificates
_generate-db-certs:
    cd backend && TEST_CERT_DIR_NAME=test_certs scripts/generate_test_certs.sh

# Use the current test results as new reference images.
update-flutter-goldens:
    fvm flutter test --update-goldens

run-client *args='':
    cd app && fvm flutter run {{args}}

run-client-no-rebuild device="macos":
    #!/usr/bin/env -S bash -eu
    app/build/{{device}}/Build/Products/Debug/Air.app/Contents/*/Air

run-server:
    cargo run --bin airserver | bunyan
