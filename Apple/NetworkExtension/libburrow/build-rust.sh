#!/bin/bash

# This is a build script. It is run by Xcode as a build step.
# The type of build is described in various environment variables set by Xcode.

export PATH="${PATH}:${HOME}/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:/etc/profiles/per-user/${USER}/bin"

if ! [[ -x "$(command -v cargo)" ]]; then
    echo 'error: Unable to find cargo'
    exit 127
fi

set -e

# Change directories relative to the location of this script
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"/../../../burrow

RUST_TARGETS=()

# Match the PLATFORM_NAME (iphoneos) and ARCHS (arm64) to a set of RUST_TARGETS (aarch64-apple-ios)
IFS=' ' read -a BURROW_ARCHS <<< "${ARCHS[@]}"
for ARCH in "${BURROW_ARCHS[@]}"; do
    case $PLATFORM_NAME in
        iphonesimulator)
            case $ARCH in
                arm64) RUST_TARGETS+=("aarch64-apple-ios-sim") ;;
                x86_64) RUST_TARGETS+=("x86_64-apple-ios") ;;
                *) echo "error: Unknown $PLATFORM_NAME arch, $ARCH"; exit 1 ;;
            esac
            ;;
        iphoneos)
            case $ARCH in
                arm64) RUST_TARGETS+=("aarch64-apple-ios") ;;
                *) echo "error: Unknown $PLATFORM_NAME arch, $ARCH"; exit 1 ;;
            esac
            ;;
        macos*)
            case $ARCH in
                arm64) RUST_TARGETS+=("aarch64-apple-darwin") ;;
                x86_64) RUST_TARGETS+=("x86_64-apple-darwin") ;;
                *) echo "error: Unknown $PLATFORM_NAME arch, $ARCH"; exit 1 ;;
            esac
            ;;
        *) echo "error: Unsupported platform $PLATFORM_NAME"; exit 1 ;;
    esac
done

# Pass all RUST_TARGETS in a single invocation
CARGO_ARGS=()
for TARGET in "${RUST_TARGETS[@]}"; do
    CARGO_ARGS+=("--target")
    CARGO_ARGS+=("$TARGET")
done

CARGO_ARGS+=("--lib")

# Pass the configuration (Debug or Release) through to cargo
if [[ $SWIFT_ACTIVE_COMPILATION_CONDITIONS == *DEBUG* ]]; then
    CARGO_TARGET_SUBDIR="debug"
else
    CARGO_ARGS+=("--release")
    CARGO_TARGET_SUBDIR="release"
fi

if [[ -x "$(command -v rustup)" ]]; then
    CARGO_PATH="$(dirname $(rustup which cargo)):/usr/bin"
else
    CARGO_PATH="$(dirname $(readlink -f $(which cargo))):/usr/bin"
fi

PROTOC=$(readlink -f $(which protoc))
CARGO_PATH="$(dirname $PROTOC):$CARGO_PATH"

# Run cargo without the various environment variables set by Xcode.
# Those variables can confuse cargo and the build scripts it runs.
env -i PATH="$CARGO_PATH" PROTOC="$PROTOC" CARGO_TARGET_DIR="${CONFIGURATION_TEMP_DIR}/target" IPHONEOS_DEPLOYMENT_TARGET="$IPHONEOS_DEPLOYMENT_TARGET" MACOSX_DEPLOYMENT_TARGET="$MACOSX_DEPLOYMENT_TARGET" cargo build "${CARGO_ARGS[@]}"

mkdir -p "${BUILT_PRODUCTS_DIR}"

# Use `lipo` to merge the architectures together into BUILT_PRODUCTS_DIR
/usr/bin/xcrun --sdk $PLATFORM_NAME lipo \
    -create $(printf "${CONFIGURATION_TEMP_DIR}/target/%q/${CARGO_TARGET_SUBDIR}/libburrow.a " "${RUST_TARGETS[@]}") \
    -output "${BUILT_PRODUCTS_DIR}/libburrow.a"
