# Linux GTK App Getting Started

## Dependencies

### Install Build Dependencies

<details>
  <summary>Debian</summary>

  > Note: Burrow currently cannot compile on Debian Stable (Bookworm) due to its outdated dependencies

  1. Install build dependencies

  ```
  sudo apt install -y clang meson cmake pkg-config libgtk-4-dev libadwaita-1-dev gettext desktop-file-utils
  ```

  2. Install flatpak builder (Optional)

  ```
  sudo apt install -y flatpak-builder
  ```

</details>

<details>
  <summary>Fedora</summary>

  1. Install build dependencies

  ```
  sudo dnf install -y clang ninja cmake meson gtk4-devel glib2-devel libadwaita-devel desktop-file-utils libappstream-glib
  ```

  2. Install flatpak builder (Optional)

  ```
  sudo dnf install -y flatpak-builder
  ```

</details>

<details>
  <summary>Void Linux (glibc)</summary>

  1. Install build dependencies

  ```
  sudo xbps-install -Sy gcc clang meson cmake pkg-config gtk4-devel gettext desktop-file-utils gtk4-update-icon-cache appstream-glib
  ```

  2. Install flatpak builder (Optional)

  ```
  sudo xbps-install -Sy flatpak-builder
  ```

</details>

### Flatpak Build Dependencies (Optional)

```
flatpak install --user \
    org.gnome.Platform/x86_64/45 \
    org.freedesktop.Sdk.Extension.rust-stable/x86_64/23.08
```

## Building

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


## Running

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
