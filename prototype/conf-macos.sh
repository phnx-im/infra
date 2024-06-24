#!/bin/bash

# SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# === Window style ===

# Set the titlebarAppearsTransparent, titleVisibility and fullSizeContentView attributes to the MainMenu.xib file
# This will make the titlebar transparent, hide the title and make the content view full size

# Path to the MainMenu.xib file
MAINMENU_XIB_PATH="macos/Runner/Base.lproj/MainMenu.xib"

# Install the required tools
brew install xmlstarlet &> /dev/null;

# Modify the XML using xmlstarlet
xmlstarlet ed --inplace \
    -i '//window' -t attr -n titlebarAppearsTransparent -v YES \
    -i '//window' -t attr -n titleVisibility -v hidden \
    -i '//window/windowStyleMask' -t attr -n fullSizeContentView -v YES \
    "$MAINMENU_XIB_PATH"

# === Entitlements ===

# Path to the release entitlements file
RELEASE_ENTITLEMENTS_PATH="macos/Runner/Release.entitlements"

# Path to the debug entitlements file
DEBUG_ENTITLEMENTS_PATH="macos/Runner/DebugProfile.entitlements"

# Add network client entitlement to release entitlements
# Enables network client access
/usr/libexec/PlistBuddy -c "Add :com.apple.security.network.client bool true" "$RELEASE_ENTITLEMENTS_PATH"

# Add network client entitlement to debug entitlements
# Enables network client access
/usr/libexec/PlistBuddy -c "Add :com.apple.security.network.client bool true" "$DEBUG_ENTITLEMENTS_PATH"

# Add file selector entitlement to release entitlements
# Allows read-only access to user-selected files
/usr/libexec/PlistBuddy -c "Add :com.apple.security.files.user-selected.read-only bool true" "$RELEASE_ENTITLEMENTS_PATH"

# Add file selector entitlement to debug entitlements
# Allows read-only access to user-selected files
/usr/libexec/PlistBuddy -c "Add :com.apple.security.files.user-selected.read-only bool true" "$DEBUG_ENTITLEMENTS_PATH"
