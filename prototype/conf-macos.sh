#!/bin/sh

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

echo "MainMenu.xib modified successfully"
