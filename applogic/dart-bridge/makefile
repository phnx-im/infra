# SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

.SILENT:
.DEFAULT_GOAL := help
PROJECTNAME=$(shell basename "$(PWD)")

OS_NAME=$(shell uname | tr '[:upper:]' '[:lower:]')

RUST_BASE_DIR := ..
RUST_TARGET_DIR := $(RUST_BASE_DIR)/../target
DART_BASE_DIR := lib/src
LIB_NAME=applogic

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

setup-macos:
	brew install --cask flutter
	brew install gawk
	make setup-flutter
	flutter config --enable-macos-desktop
	cargo install flutter_rust_bridge_codegen
	cargo install cargo-expand

setup-linux:
	sudo snap install flutter --classic
	sudo apt-get install clang cmake ninja-build pkg-config libgtk-3-dev gawk
	make setup-flutter
	flutter config --enable-linux-desktop
	make setup-rust

setup-ci: setup-flutter setup-rust

setup-flutter:
	flutter channel stable
	flutter upgrade
	flutter pub get
	flutter doctor
	export PATH="$$PATH:$$HOME/.pub-cache/bin"

setup-rust:
	rustup upgrade
	cd ../.. ; cargo clean ; cargo update
	cargo install flutter_rust_bridge_codegen
	cargo install cargo-expand

# coreclient

macos-debug:
	cd $(RUST_BASE_DIR) ; \
		cargo build --target x86_64-apple-darwin && \
		cargo build --target aarch64-apple-darwin
	mkdir -p macos
	lipo -create \
		$(RUST_TARGET_DIR)/x86_64-apple-darwin/debug/libphnx$(LIB_NAME).a \
		$(RUST_TARGET_DIR)/aarch64-apple-darwin/debug/libphnx$(LIB_NAME).a \
		-output macos/libphnx$(LIB_NAME).a

macos-release:
	cd $(RUST_BASE_DIR) ; \
		cargo build --target x86_64-apple-darwin --release && \
		cargo build --target aarch64-apple-darwin --release
	mkdir -p macos
	lipo -create \
		$(RUST_TARGET_DIR)/x86_64-apple-darwin/release/libphnx$(LIB_NAME).a \
		$(RUST_TARGET_DIR)/aarch64-apple-darwin/release/libphnx$(LIB_NAME).a \
		-output macos/libphnx$(LIB_NAME).a

ios-debug:
	cd $(RUST_BASE_DIR) ; \
		cargo build --target aarch64-apple-ios && \
		cargo build --target aarch64-apple-ios-sim && \
		cargo build --target x86_64-apple-ios
	mkdir -p ios
	lipo -create \
		$(RUST_TARGET_DIR)/aarch64-apple-ios-sim/debug/libphnx$(LIB_NAME).a \
		$(RUST_TARGET_DIR)/x86_64-apple-ios/debug/libphnx$(LIB_NAME).a \
		-output ios/libphnx$(LIB_NAME).a
	rm -rf ios/libphnx$(LIB_NAME).xcframework
	xcodebuild -create-xcframework \
		-library $(RUST_TARGET_DIR)/aarch64-apple-ios/debug/libphnx$(LIB_NAME).a \
		-library ios/libphnx$(LIB_NAME).a \
		-output ios/libphnx$(LIB_NAME).xcframework
	rm -rf ios/libphnx$(LIB_NAME).a

ios-release:
	cd $(RUST_BASE_DIR) ; \
		cargo build --target aarch64-apple-ios --release && \
		cargo build --target aarch64-apple-ios-sim --release && \
		cargo build --target x86_64-apple-ios --release
	mkdir -p ios
	lipo -create \
		$(RUST_TARGET_DIR)/aarch64-apple-ios-sim/release/libphnx$(LIB_NAME).a \
		$(RUST_TARGET_DIR)/x86_64-apple-ios/release/libphnx$(LIB_NAME).a \
		-output ios/libphnx$(LIB_NAME).a
	rm -rf ios/libphnx$(LIB_NAME).xcframework
	xcodebuild -create-xcframework \
		-library $(RUST_TARGET_DIR)/aarch64-apple-ios/release/libphnx$(LIB_NAME).a \
		-library ios/libphnx$(LIB_NAME).a \
		-output ios/libphnx$(LIB_NAME).xcframework
	rm -rf ios/libphnx$(LIB_NAME).a

android-debug:
	cd $(RUST_BASE_DIR) ; \
		cargo ndk -t arm64-v8a build
	mkdir -p android/app/src/main/jniLibs/arm64-v8a
	cp $(RUST_TARGET_DIR)/aarch64-linux-android/debug/libphnx$(LIB_NAME).so \
		android/app/src/main/jniLibs/arm64-v8a/libphnx$(LIB_NAME).so

android-release:
	cd $(RUST_BASE_DIR) ; \
		cargo ndk -t arm64-v8a build --release
	mkdir -p android/app/src/main/jniLibs/arm64-v8a
	cp $(RUST_TARGET_DIR)/aarch64-linux-android/release/libphnx$(LIB_NAME).so \
		android/app/src/main/jniLibs/arm64-v8a/libphnx$(LIB_NAME).so

linux-debug:
	cd $(RUST_BASE_DIR) ; \
		cargo build --target x86_64-unknown-linux-gnu
	mkdir -p linux
	cp $(RUST_TARGET_DIR)/debug/libphnx$(LIB_NAME).so linux/libphnx$(LIB_NAME).so

dart-bridge:
	touch bridge_generated.rs
	flutter pub get
	flutter_rust_bridge_codegen \
	 	--rust-input ../src/dart_api.rs \
        --dart-output $(DART_BASE_DIR)/bridge_generated.dart \
        --dart-decl-output $(DART_BASE_DIR)/bridge_definitions.dart \
    	--c-output bridge_generated.h \
		--rust-output bridge_generated.rs \
		--inline-rust \
        --class-name=RustBridge \
        --skip-add-mod-to-lib
