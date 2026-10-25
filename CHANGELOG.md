# Changelog

## 0.1.7 — 2026-10-25

- Portable x86_64 AppImage built against Ubuntu 22.04 / glibc 2.35.
- Pinned GTK/libadwaita and supporting libraries built from source on that baseline.
- Image loaders and their dependencies included for reliable icon/SVG loading.
- Packaging rejects bundled ELF files requiring a newer glibc.
- Clean-container compatibility checks and a portable-build CI workflow.
- Per-device settings and native input behavior are unchanged.

## 0.1.6 — 2026-10-25

Initial public preview.

- Native GTK4/libadwaita desktop app and browser touchpad.
- Smooth multitouch input interpreted by libinput and GNOME.
- QR pairing, device management, one active controller, and input watchdog.
- Desktop-persisted device settings and independent cursor/scroll speeds.
- Full-screen controller with compact menu and theme/touch indicator options.
- Tray menu, background operation, and optional start at login.
- Continuous full-surface movement above 1× speed; balanced horizontal/vertical
  motion on portrait and landscape screens.
- Fedora 44 x86_64 AppImage, dependency source bundle, and checksums.

Known limits: HTTP/WebSocket traffic is unencrypted; trusted LAN use only. The
packaged binary is tested on Fedora 44 / GNOME 50 / Wayland. Other environments
need testing, and older distributions may require a source build.
