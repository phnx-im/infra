#!/bin/bash

# SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Path to the Info.plist file
INFO_PLIST_PATH="ios/Runner/Info.plist"

# Add or update the ITSAppUsesNonExemptEncryption key
# This key indicates whether the app uses non-exempt encryption.
# Setting it to false means the app does not use encryption that is subject to export compliance.
/usr/libexec/PlistBuddy -c "Add :ITSAppUsesNonExemptEncryption bool false" "$INFO_PLIST_PATH"

# Add or update the LSApplicationCategoryType key
# This key defines the category of the app as it appears in the App Store.
# Setting it to public.app-category.social-networking categorizes the app under Social Networking.
/usr/libexec/PlistBuddy -c "Add :LSApplicationCategoryType string public.app-category.social-networking" "$INFO_PLIST_PATH"

# Add or update the UIBackgroundModes key
# This key specifies the background tasks the app supports.
# Adding remote-notification allows the app to receive remote notifications while in the background.
/usr/libexec/PlistBuddy -c "Add :UIBackgroundModes array" "$INFO_PLIST_PATH"
/usr/libexec/PlistBuddy -c "Add :UIBackgroundModes:0 string remote-notification" "$INFO_PLIST_PATH"

# Add or update the NSCameraUsageDescription key
# This key provides a description to the user on why the app needs access to the camera.
# The given description will be displayed when the app requests camera access.
/usr/libexec/PlistBuddy -c "Add :NSCameraUsageDescription string 'Access to the camera is required to take a picture that can be used as a profile picture'" "$INFO_PLIST_PATH"

# Add or update the NSPhotoLibraryUsageDescription key
# This key provides a description to the user on why the app needs access to the photo library.
# The given description will be displayed when the app requests photo library access.
/usr/libexec/PlistBuddy -c "Add :NSPhotoLibraryUsageDescription string 'Access to the photo library is required to set a profile picture'" "$INFO_PLIST_PATH"

# Add or update the UIViewControllerBasedStatusBarAppearance key
# This key indicates whether the status bar appearance is controlled by individual view controllers.
# Setting it to false means the app will globally control the status bar appearance.
/usr/libexec/PlistBuddy -c "Add :UIViewControllerBasedStatusBarAppearance bool false" "$INFO_PLIST_PATH"

# Modify the UISupportedInterfaceOrientations key to contain only one array element
# This key specifies the supported interface orientations for the app.
# Limiting it to UIInterfaceOrientationPortrait means the app only supports portrait mode.
/usr/libexec/PlistBuddy -c "Delete :UISupportedInterfaceOrientations" "$INFO_PLIST_PATH"
/usr/libexec/PlistBuddy -c "Add :UISupportedInterfaceOrientations array" "$INFO_PLIST_PATH"
/usr/libexec/PlistBuddy -c "Add :UISupportedInterfaceOrientations:0 string UIInterfaceOrientationPortrait" "$INFO_PLIST_PATH"
