#!/bin/bash

set -ex

BURROW_GTK_ROOT="$(readlink -f $(dirname -- "$(readlink -f -- "$BASH_SOURCE")")/..)"
BURROW_GTK_BUILD="$BURROW_GTK_ROOT/build-appimage"
LINUXDEPLOY_VERSION="${LINUXDEPLOY_VERSION:-"1-alpha-20240109-1"}"
BURROW_BUILD_TYPE="${BURROW_BUILD_TYPE:-"release"}"
HOST_ARCHITECTURE=$(lscpu | grep Architecture | awk '{print $2}')
TARGET_ARCHITECTURE="${TARGET_ARCHITECTURE:-"x86_64"}"
CARGO_FLAGS=""

if [ "$BURROW_GTK_ROOT" != $(pwd) ]; then
    echo "Make sure to cd into burrow-gtk"
    exit 1
fi

if [ "$HOST_ARCHITECTURE" == "x86_64" ]; then
    wget "https://github.com/linuxdeploy/linuxdeploy/releases/download/$LINUXDEPLOY_VERSION/linuxdeploy-x86_64.AppImage" -o /dev/null -O /tmp/linuxdeploy
    chmod a+x /tmp/linuxdeploy
elif [ "$HOST_ARCHITECTURE" == "aarch64" ]; then
    wget "https://github.com/linuxdeploy/linuxdeploy/releases/download/$LINUXDEPLOY_VERSION/linuxdeploy-aarch64.AppImage" -o /dev/null -O /tmp/linuxdeploy
    chmod a+x /tmp/linuxdeploy
fi

if [ "$TARGET_ARCHITECTURE" == "x86_64" ]; then
    CARGO_FLAGS="--target x86_64-unknown-linux-gnu"
elif [ "$TARGET_ARCHITECTURE" == "aarch64" ]; then
    CARGO_FLAGS="--target aarch64-unknown-linux-gnu"
fi

CFLAGS="-I/usr/local/include -I/usr/include/$MUSL_TARGET -fPIE"
meson setup $BURROW_GTK_BUILD --bindir bin --prefix /usr --buildtype $BURROW_BUILD_TYPE
meson compile -C $BURROW_GTK_BUILD
DESTDIR=AppDir meson install -C $BURROW_GTK_BUILD
CARGO_FLAGS=$CARGO_FLAGS cargo b --$BURROW_BUILD_TYPE --manifest-path=../Cargo.toml
/tmp/linuxdeploy --appimage-extract-and-run --appdir $BURROW_GTK_BUILD/AppDir -e $BURROW_GTK_BUILD/../../target/$BURROW_BUILD_TYPE/burrow --output appimage
mv *.AppImage $BURROW_GTK_BUILD/Burrow_${TARGET_ARCHITECTURE}
