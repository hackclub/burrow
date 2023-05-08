#!/bin/bash

export PATH="${PATH}:${HOME}/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:/etc/profiles/per-user/${USER}/bin"

if ! [[ -x "$(command -v cargo)" ]]; then
    echo 'error: Unable to find cargo'
    exit 127
fi

set -e

cd -- "$(dirname -- "${BASH_SOURCE[0]}")"/../../../burrow

RUST_TARGETS=()

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

CARGO_ARGS=()
for TARGET in "${RUST_TARGETS[@]}"; do
    CARGO_ARGS+=("--target")
    CARGO_ARGS+=("$TARGET")
done

CARGO_ARGS+=("--lib")

if [[ $SWIFT_ACTIVE_COMPILATION_CONDITIONS == *DEBUG* ]]; then
    CARGO_DIR="debug"
else
    CARGO_ARGS+=("--release")
    CARGO_DIR="release"
fi

if [[ -x "$(command -v rustup)" ]]; then
    CARGO_PATH="$(dirname $(rustup which cargo)):/usr/bin"
else
    CARGO_PATH="$(dirname $(readlink -f $(which cargo))):/usr/bin"
fi

env -i PATH="$CARGO_PATH" cargo build "${CARGO_ARGS[@]}"

mkdir -p "${BUILT_PRODUCTS_DIR}"
/usr/bin/xcrun --sdk $PLATFORM_NAME lipo \
    -create $(printf "${PROJECT_DIR}/../target/%q/${CARGO_DIR}/libburrow.a " "${RUST_TARGETS[@]}") \
    -output "${BUILT_PRODUCTS_DIR}/libburrow.a"
