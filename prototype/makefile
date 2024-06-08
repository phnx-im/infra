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

frb-integrate:
	flutter_rust_bridge_codegen integrate --rust-crate-name phnxapplogic --rust-crate-dir ../applogic

setup-ci:
	curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
	cargo binstall -y flutter_rust_bridge_codegen@2.0.0-dev.37
	cargo binstall -y cargo-expand
	cargo binstall -y cargo-ndk

dart-bridge:
	rm -f $(RUST_BASE_DIR)/src/frb_*.rs
	touch $(RUST_BASE_DIR)/src/frb_generated.rs
	mkdir -p lib/core
	rm -Rf lib/core/*
	flutter pub get
	flutter_rust_bridge_codegen generate

android-debug:
	flutter build apk --debug

android-release:
	flutter build apk --release



