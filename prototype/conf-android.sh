#!/bin/bash

# SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# === Restrict target architectures ===

# Path to the build.gradle file
BUILD_GRADLE_PATH="android/app/build.gradle"

# ABI filter to add
ABI_FILTERS=("arm64-v8a")

 if grep -q "abiFilters" "$BUILD_GRADLE_PATH"; then
        echo "ABI filters already present in build.gradle."
    else
        echo "Adding ABI filters to build.gradle."

        # Create the ABI filter string
        abi_filter="        ndk {\n            abiFilters "
        for abi in "${ABI_FILTERS[@]}"; do
            abi_filter+="\"$abi\", "
        done
        abi_filter="${abi_filter%, }\n        }\n"

        # Use awk to add ABI filter to defaultConfig
        awk -v abi_filter="$abi_filter" '
        /defaultConfig {/ {
            print
            print abi_filter
            next
        }
        { print }
        ' "$BUILD_GRADLE_PATH" > temp.gradle && mv temp.gradle "$BUILD_GRADLE_PATH"

        echo "ABI filters added successfully."
    fi

