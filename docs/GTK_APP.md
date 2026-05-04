# Linux GTK App Getting Started

Currently, the GTK App can be built as a binary or as an AppImage.
Note that the flatpak version can compile but will not run properly!

## Dependencies

### Install Build Dependencies

<details>
  <summary>Debian</summary>

  > Note: Burrow currently cannot compile on Debian Stable (Bookworm) due to its outdated dependencies

  1. Install build dependencies

  ```
  sudo apt install -y clang meson cmake pkg-config libssl-dev libgtk-4-dev libadwaita-1-dev gettext desktop-file-utils
  ```

  2. Install flatpak builder (Optional)

  ```
  sudo apt install -y flatpak-builder
  ```

  3. Install AppImage build tools (Optional)
  
  ```
  sudo apt install -y wget fuse file
  ```

</details>

<details>
  <summary>Fedora</summary>

  1. Install build dependencies

  ```
  sudo dnf install -y clang ninja-build cmake meson openssl-devel gtk4-devel glib2-devel libadwaita-devel desktop-file-utils libappstream-glib
  ```

  2. Install flatpak builder (Optional)

  ```
  sudo dnf install -y flatpak-builder
  ```

  3. Install AppImage build tools (Optional)
  
  ```
  sudo dnf install -y util-linux wget fuse fuse-libs file
  ```

</details>

<details>
  <summary>Void Linux (glibc)</summary>

  1. Install build dependencies

  ```
  sudo xbps-install -Sy gcc clang meson cmake pkg-config openssl-devel gtk4-devel gettext desktop-file-utils gtk4-update-icon-cache appstream-glib
  ```

  2. Install flatpak builder (Optional)

  ```
  sudo xbps-install -Sy flatpak-builder
  ```

  3. Install AppImage build tools (Optional)
  
  ```
  sudo xbps-install -Sy wget fuse file
  ```

</details>

### Flatpak Build Dependencies (Optional)

```
flatpak install --user \
    org.gnome.Platform/x86_64/45 \
    org.freedesktop.Sdk.Extension.rust-stable/x86_64/23.08
```

## Building

With Nix, enter the focused GTK shell before running the Meson build:

```bash
nix develop .#gtk
```

<details>
  <summary>General</summary>

  1. Enter the `burrow-gtk`

  ```bash
  cd burrow-gtk
  ```

  2. Perform the meson build
  ```
  meson setup build
  meson compile -C build
  ```

</details>

<details>
  <summary>Flatpak</summary>

  1. Compile and install the flatpak

  ```
  flatpak-builder
      --user --install --force-clean --disable-rofiles-fuse \
      flatpak_debug/ \
      burrow-gtk/build-aux/com.hackclub.burrow.devel.json
  ```

</details>

<details>
  <summary>AppImage</summary>

  1. Enter the `burrow-gtk`

  ```bash
  cd burrow-gtk
  ```

  2. Compile the AppImage
  
  ```
  ./build-aux/build_appimage.sh
  ```

</details>


## Running

The GTK app mirrors the Apple home surface: a Burrow header, Networks carousel,
Accounts section, Tunnel action, and the same add flows for WireGuard, Tor, and
Tailnet. It talks to the daemon over the same gRPC API used by Apple clients for
network storage, tunnel state, Tailnet discovery, authority probing, browser
sign-in, and Tailnet payloads.

On Linux the GTK app first looks for a daemon on the configured gRPC socket. If
none is reachable, it starts an embedded user-scoped daemon with a socket under
`XDG_RUNTIME_DIR` and a database under `XDG_DATA_HOME` before refreshing the UI.

<details>
  <summary>General</summary>

  The compiled binary can be found in `build/src/burrow-gtk`.

  ```
  ./build/src/burrow-gtk
  ```
</details>

<details>
  <summary>Flatpak</summary>

  ```
  flatpak run com.hackclub.burrow-devel
  ```

</details>

<details>
  <summary>AppImage</summary>

  The compiled binary can be found in `build-appimage/Burrow-*.AppImage`.

  ```
  ./build-appimage/Burrow-*.AppImage
  ```

</details>
