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
    LLVM_TARGET=aarch64-unknown-linux-musl
    MUSL_TARGET=aarch64-linux-musl
elif [ "$ARCHITECTURE" == "aarch64" ]; then
    wget "https://github.com/linuxdeploy/linuxdeploy/releases/download/$LINUXDEPLOY_VERSION/linuxdeploy-aarch64.AppImage" -o /dev/null -O /tmp/linuxdeploy
    chmod a+x /tmp/linuxdeploy
    LLVM_TARGET=x86_64-unknown-linux-musl
    MUSL_TARGET=x86_64-linux-musl
fi

rustup target add $LLVM_TARGET
curl --proto '=https' --tlsv1.2 -sSfO https://www.sqlite.org/2022/sqlite-autoconf-$SQLITE_VERSION.tar.gz
tar xf sqlite-autoconf-$SQLITE_VERSION.tar.gz
rm sqlite-autoconf-$SQLITE_VERSION.tar.gz
cd sqlite-autoconf-$SQLITE_VERSION
./configure --disable-shared
    CC="clang-$LLVM_VERSION -target $LLVM_TARGET"
    CFLAGS="-I/usr/local/include -I/usr/include/$MUSL_TARGET"
    LDFLAGS="-L/usr/local/lib -L/usr/lib/$MUSL_TARGET -L/lib/$MUSL_TARGET"
make
make install
cd ..
rm -rf sqlite-autoconf-$SQLITE_VERSION

meson setup $BURROW_GTK_BUILD --bindir bin --prefix /usr --buildtype $BURROW_BUILD_TYPE
meson compile -C $BURROW_GTK_BUILD
DESTDIR=AppDir meson install -C $BURROW_GTK_BUILD
cargo b --$BURROW_BUILD_TYPE --manifest-path=../Cargo.toml
/tmp/linuxdeploy --appimage-extract-and-run --appdir $BURROW_GTK_BUILD/AppDir -e $BURROW_GTK_BUILD/../../target/$BURROW_BUILD_TYPE/burrow --output appimage
mv *.AppImage $BURROW_GTK_BUILD
