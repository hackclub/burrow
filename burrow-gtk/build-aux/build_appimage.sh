#!/bin/bash

BURROW_GTK_ROOT="$(readlink -f $(dirname -- "$(readlink -f -- "$BASH_SOURCE")")/..)"
BURROW_GTK_BUILD="$BURROW_GTK_ROOT/build-appimage"

if [ "$BURROW_GTK_ROOT" != $(pwd) ]; then
    echo "Make sure to cd into burrow-gtk"
    exit 1
fi

ARCHITECTURE=$(lscpu | grep Architecture | awk '{print $2}')

if [ "$ARCHITECTURE" == "x86_64" ]; then
    wget "https://github.com/linuxdeploy/linuxdeploy/releases/download/1-alpha-20240109-1/linuxdeploy-x86_64.AppImage" -o /dev/null -O /tmp/linuxdeploy
    chmod a+x /tmp/linuxdeploy
elif [ "$ARCHITECTURE" == "aarch64" ]; then
    wget "https://github.com/linuxdeploy/linuxdeploy/releases/download/1-alpha-20240109-1/linuxdeploy-aarch64.AppImage" -o /dev/null -O /tmp/linuxdeploy
    chmod a+x /tmp/linuxdeploy
fi

meson setup $BURROW_GTK_BUILD --bindir bin --prefix /usr
meson compile -C $BURROW_GTK_BUILD
DESTDIR=AppDir meson install -C $BURROW_GTK_BUILD
/tmp/linuxdeploy --appdir $BURROW_GTK_BUILD/AppDir --output appimage
mv *.AppImage $BURROW_GTK_BUILD
