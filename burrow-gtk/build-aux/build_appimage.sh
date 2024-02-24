#!/bin/bash

set -ex

BURROW_GTK_ROOT="$(readlink -f $(dirname -- "$(readlink -f -- "$BASH_SOURCE")")/..)"
BURROW_GTK_BUILD="$BURROW_GTK_ROOT/build-appimage"
LINUXDEPLOY_VERSION="${LINUXDEPLOY_VERSION:-"1-alpha-20240109-1"}"
BURROW_BUILD_TYPE="${BURROW_BUILD_TYPE:-"release"}"

if [ "$BURROW_GTK_ROOT" != $(pwd) ]; then
    echo "Make sure to cd into burrow-gtk"
    exit 1
fi

ARCHITECTURE=$(lscpu | grep Architecture | awk '{print $2}')

if [ "$ARCHITECTURE" == "x86_64" ]; then
    wget "https://github.com/linuxdeploy/linuxdeploy/releases/download/$LINUXDEPLOY_VERSION/linuxdeploy-x86_64.AppImage" -o /dev/null -O /tmp/linuxdeploy
    chmod a+x /tmp/linuxdeploy
elif [ "$ARCHITECTURE" == "aarch64" ]; then
    wget "https://github.com/linuxdeploy/linuxdeploy/releases/download/$LINUXDEPLOY_VERSION/linuxdeploy-aarch64.AppImage" -o /dev/null -O /tmp/linuxdeploy
    chmod a+x /tmp/linuxdeploy
fi

meson setup $BURROW_GTK_BUILD --bindir bin --prefix /usr --buildtype $BURROW_BUILD_TYPE
meson compile -C $BURROW_GTK_BUILD
DESTDIR=AppDir meson install -C $BURROW_GTK_BUILD
cargo b --$BURROW_BUILD_TYPE --manifest-path=../Cargo.toml
/tmp/linuxdeploy --appimage-extract-and-run --appdir $BURROW_GTK_BUILD/AppDir -e $BURROW_GTK_BUILD/../../target/$BURROW_BUILD_TYPE/burrow --output appimage
mv *.AppImage $BURROW_GTK_BUILD
