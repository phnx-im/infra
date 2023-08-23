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
	cargo install flutter_rust_bridge_codegen cargo-expand

setup-linux:
	sudo snap install flutter --classic
	sudo apt-get install clang cmake ninja-build pkg-config libgtk-3-dev gawk
	make setup-flutter
	flutter config --enable-linux-desktop
	cargo install flutter_rust_bridge_codegen cargo-expand

setup-ci:
	flutter pub get
	flutter doctor
	cargo install flutter_rust_bridge_codegen cargo-expand --force

setup-flutter:
	flutter upgrade
	flutter pub get
	flutter doctor
	export PATH="$$PATH:$$HOME/.pub-cache/bin"

# coreclient

macos-debug:
	cd $(RUST_BASE_DIR) ; \
		cargo build --features="dart-bridge" --target x86_64-apple-darwin && \
		cargo build --features="dart-bridge" --target aarch64-apple-darwin
	mkdir -p macos
	lipo -create \
		$(RUST_TARGET_DIR)/x86_64-apple-darwin/debug/libphnxcoreclient.a \
		$(RUST_TARGET_DIR)/aarch64-apple-darwin/debug/libphnxcoreclient.a \
		-output macos/libphnxcoreclient.a

macos-release:
	cd $(RUST_BASE_DIR) ; \
		cargo build --features="dart-bridge" --target x86_64-apple-darwin --release && \
		cargo build --features="dart-bridge" --target aarch64-apple-darwin --release
	mkdir -p macos
	lipo -create \
		$(RUST_TARGET_DIR)/x86_64-apple-darwin/release/libphnxcoreclient.a \
		$(RUST_TARGET_DIR)/aarch64-apple-darwin/release/libphnxcoreclient.a \
		-output macos/libphnxcoreclient.a

ios-debug:
	cd $(RUST_BASE_DIR) ; \
		cargo build --features="dart-bridge" --target aarch64-apple-ios && \
		cargo build --features="dart-bridge" --target aarch64-apple-ios-sim && \
		cargo build --features="dart-bridge" --target x86_64-apple-ios
	mkdir -p ios
	lipo -create \
		$(RUST_TARGET_DIR)/aarch64-apple-ios-sim/debug/libphnxcoreclient.a \
		$(RUST_TARGET_DIR)/x86_64-apple-ios/debug/libphnxcoreclient.a \
		-output ios/libphnxcoreclient.a
	rm -rf ios/libphnxcoreclient.xcframework
	xcodebuild -create-xcframework \
		-library $(RUST_TARGET_DIR)/aarch64-apple-ios/debug/libphnxcoreclient.a \
		-library ios/libphnxcoreclient.a \
		-output ios/libphnxcoreclient.xcframework
	rm -rf ios/libphnxcoreclient.a

ios-release:
	cd $(RUST_BASE_DIR) ; \
		cargo build --features="dart-bridge" --target aarch64-apple-ios --release && \
		cargo build --features="dart-bridge" --target aarch64-apple-ios-sim --release && \
		cargo build --features="dart-bridge" --target x86_64-apple-ios --release
	mkdir -p ios
	lipo -create \
		$(RUST_TARGET_DIR)/aarch64-apple-ios-sim/release/libphnxcoreclient.a \
		$(RUST_TARGET_DIR)/x86_64-apple-ios/release/libphnxcoreclient.a \
		-output ios/libphnxcoreclient.a
	rm -rf ios/libphnxcoreclient.xcframework
	xcodebuild -create-xcframework \
		-library $(RUST_TARGET_DIR)/aarch64-apple-ios/release/libphnxcoreclient.a \
		-library ios/libphnxcoreclient.a \
		-output ios/libphnxcoreclient.xcframework
	rm -rf ios/libphnxcoreclient.a

android-debug:
	cd $(RUST_BASE_DIR) ; \
		cargo build --features="dart-bridge" --target aarch64-linux-android
	cp $(RUST_TARGET_DIR)/aarch64-linux-android/debug/libphnxcoreclient.so android/src/main/jniLibs/arm64-v8a/libphnxcoreclient.so

android-release:
	cd $(RUST_BASE_DIR) ; \
		cargo build --features="dart-bridge" --target aarch64-linux-android --release
	cp $(RUST_TARGET_DIR)/aarch64-linux-android/release/libphnxcoreclient.so android/src/main/jniLibs/arm64-v8a/libphnxcoreclient.so

linux-debug:
	cd $(RUST_BASE_DIR) ; \
		cargo build --features="dart-bridge" --target x86_64-unknown-linux-gnu
	mkdir -p linux
	cp $(RUST_TARGET_DIR)/debug/libphnxcoreclient.so linux/libphnxcoreclient.so

dart-bridge:
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
