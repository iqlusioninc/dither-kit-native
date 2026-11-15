#!/bin/sh
set -eu
source_file=$1
library_file=$2
work_directory=$(dirname "$library_file")
air_file="$work_directory/Dither.air"
sdk=${SDKROOT:-macosx}
case "$sdk" in
    *iPhoneSimulator*|iphonesimulator*) deployment=-mios-simulator-version-min=17.0 ;;
    *iPhoneOS*|iphoneos*) deployment=-mios-version-min=17.0 ;;
    *MacOSX*|macosx*) deployment=-mmacosx-version-min=14.0 ;;
    *) echo "Unsupported DitherKit Metal SDK: $sdk" >&2; exit 1 ;;
esac
# SDKROOT is read here, during command execution, because Xcode does not
# provide destination settings while generating the build-plugin commands.
# Keep Clang's module cache inside the plugin's writable sandbox directory.
mkdir -p "$work_directory/ModuleCache"
trap 'rm -f "$air_file"' EXIT
printf 'Compiling Dither Metal for %s (%s)\n' "$sdk" "$deployment"
/usr/bin/xcrun --sdk "$sdk" metal -c -std=metal3.0 -fno-fast-math "$deployment" \
    -fmodules-cache-path="$work_directory/ModuleCache" "$source_file" -o "$air_file"
/usr/bin/xcrun --sdk "$sdk" metallib "$air_file" -o "$library_file"
