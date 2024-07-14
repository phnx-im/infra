# SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

.SILENT:
.DEFAULT_GOAL := help
PROJECTNAME=$(shell basename "$(PWD)")

OS_NAME=$(shell uname | tr '[:upper:]' '[:lower:]')

LIB_NAME := applogic
RUST_BASE_DIR := ../$(LIB_NAME)

.PHONY: help
help: makefile
	@echo
	@echo " Available actions in "$(PROJECTNAME)":"
	@echo
	@sed -n 's/^##//p' $< | column -t -s ':' |  sed -e 's/^/ /'
	@echo

# clean:
.PHONY: clean
clean:
	flutter clean
	dart pub get
	dart pub upgrade
	cd $(RUST_BASE_DIR) ; cargo clean

# Define a target that checks for uncommitted changes
check-git-clean:
	@if ! git diff-index --quiet HEAD --; then \
		echo "Error: You have uncommitted changes. Please commit or stash them before proceeding."; \
		exit 1; \
	fi

setup-ci:
	rustup update
	curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
	cargo binstall -y flutter_rust_bridge_codegen@2.0.0
	cargo binstall -y cargo-expand
	cargo binstall -y cargo-ndk

setup-macos-ci:
	cd fastlane && bundle install
	cd fastlane && bundle exec fastlane add_plugin flutter

frb-integrate:
	mv flutter_rust_bridge.yaml flutter_rust_bridge.yaml.tmp
	rm -Rf  rust_builder test_driver
	flutter_rust_bridge_codegen integrate --rust-crate-name phnxapplogic --rust-crate-dir ../applogic
	git restore --source=HEAD --staged --worktree ../applogic lib
	git clean -fd ../applogic lib
	mv flutter_rust_bridge.yaml flutter_rust_bridge.yaml.generated.tmp
	echo "# This is only to inspect the generated flutter_rust_bridge.yaml file. Remove if not needed.\n" > /tmp/header.tmp
	cat /tmp/header.tmp flutter_rust_bridge.yaml.generated.tmp > flutter_rust_bridge.yaml.generated
	mv flutter_rust_bridge.yaml.tmp flutter_rust_bridge.yaml
	rm flutter_rust_bridge.yaml.generated.tmp
	make frb-generate

frb-generate:
	rm -f $(RUST_BASE_DIR)/src/frb_*.rs
	touch $(RUST_BASE_DIR)/src/frb_generated.rs
	mkdir -p lib/core
	rm -Rf lib/core/*
	flutter pub get
	flutter_rust_bridge_codegen generate

android-create:
	rm -Rf android/
	flutter create --platforms=android .
	./conf-android.sh

macos-create:
	rm -Rf macos/
	flutter create --platforms=macos --org im.phnx .
	./conf-macos.sh

ios-create:
	rm -Rf ios/
	flutter create --platforms=ios --org im.phnx .

android-debug:
	flutter build apk --debug

android-release:
	flutter build apk --release

ios-release:
	flutter build ios --release



