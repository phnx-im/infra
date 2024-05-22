# SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

.SILENT:
.DEFAULT_GOAL := help
PROJECTNAME=$(shell basename "$(PWD)")

OS_NAME=$(shell uname | tr '[:upper:]' '[:lower:]')

LIB_NAME := applogic
RUST_BASE_DIR := ../$(LIB_NAME)
DART_BRIDGE_DIR := ../$(LIB_NAME)/dart-bridge

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

setup-flutter:
	# Install Flutter using git.
	git clone https://github.com/flutter/flutter.git --depth 1 -b stable $HOME/flutter
	export PATH="$PATH:$HOME/flutter/bin"
	flutter channel stable
	flutter pub get

setup-ci: setup-flutter
	cd $(DART_BRIDGE_DIR) ; make setup-ci

dart-bridge:
	 cd $(DART_BRIDGE_DIR) ; make dart-bridge

android-debug:
	cd $(DART_BRIDGE_DIR) ; make dart-bridge && make android-debug
	mkdir -p android/app/src/main/jniLibs/arm64-v8a/
	cp $(DART_BRIDGE_DIR)/android/app/src/main/jniLibs/arm64-v8a/libphnx$(LIB_NAME).so android/app/src/main/jniLibs/arm64-v8a/
	flutter build apk

android-production:
	cd $(DART_BRIDGE_DIR) ; make dart-bridge && make android-production
	mkdir -p android/app/src/main/jniLibs/arm64-v8a/
	cp $(DART_BRIDGE_DIR)/android/app/src/main/jniLibs/arm64-v8a/libphnx$(LIB_NAME).so android/app/src/main/jniLibs/arm64-v8a/
	flutter build apk --release

android-armv7a-debug:
	cd $(DART_BRIDGE_DIR) ; make dart-bridge && make android-armv7a-debug
	mkdir -p android/app/src/main/jniLibs/armeabi-v7a/
	cp $(DART_BRIDGE_DIR)/android/app/src/main/jniLibs/armeabi-v7a/libphnx$(LIB_NAME).so android/app/src/main/jniLibs/armeabi-v7a/
	flutter build apk

android-armv7a-production:
	cd $(DART_BRIDGE_DIR) ; make dart-bridge && make android-armv7a-production
	mkdir -p android/app/src/main/jniLibs/armeabi-v7a/
	cp $(DART_BRIDGE_DIR)/android/app/src/main/jniLibs/armeabi-v7a/libphnx$(LIB_NAME).so android/app/src/main/jniLibs/armeabi-v7a/
	flutter build apk --release

