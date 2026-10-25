# AppImage compatibility — 0.1.8

The release is built on Ubuntu 22.04 for x86_64. Every bundled ELF passed a symbol
version audit: no numeric GLIBC requirement above **2.35**, and no private or
newer named glibc ABI requirements. The same artifact was exercised on the
following userspaces on 2026-10-25:

| Environment | Version/startup | Pairing and network rules | Saved settings | GTK window under Xvfb |
| --- | --- | --- | --- | --- |
| Ubuntu 22.04 | Pass | Pass | Pass | Pass |
| Ubuntu 24.04 | Pass | Pass | Pass | Pass |
| Debian 12 | Pass | Pass | Pass | Pass |
| Fedora 44 | Pass | Pass | Pass | Pass |

These are clean runtime containers, without the build environment's GTK stack.
The test extracts the actual AppImage and runs its AppRun launcher. Network
checks cover origin validation, pairing, token hashing, one active controller,
invalid input, timeout, reconnect, and rate limiting. Settings checks cover
restart/new-port persistence, profile restoration, and credential rotation.
The Ubuntu window screenshot was visually checked for text and icon rendering.

For 0.1.8, native input tests cover three-finger swipe/reversal, two-finger
scrolling, pinch, watchdog release, native one/two/three-finger taps, and all five
media keys with matched press/release events. Tests exclusively grab newly created
virtual devices, so they do not change playback, volume, or the desktop cursor.
Browser checks verified fresh pairing, reload/reconnect and media-mode startup.

The 0.1.7 baseline additionally passed host GNOME Wayland preview startup,
full-surface motion/scroll at 1× and 2×, and equal-axis motion in portrait and
landscape. This release retains that input mapping and bundled UI stack.

## What this does and does not establish

- The distro matrix establishes library loading, GTK startup, and application
  behavior on each userspace. It is not a full GNOME gesture test on each distro.
- Xvfb has no accelerated DRI3 device; Mesa/EGL fallback warnings in that test are
  expected. Real graphics drivers remain supplied by the host.
- Host GL/EGL and C++ runtime libraries are not replaced by older bundled copies,
  avoiding conflicts with newer Mesa/LLVM drivers. A normal graphical Linux
  installation with those runtime libraries and fonts is expected.
- The current build targets glibc-based x86_64 desktops. ARM, Alpine/musl, and
  glibc older than 2.35 need different builds.
- `/dev/uinput` permission setup remains necessary. Native gesture availability
  depends on libinput and the desktop compositor.
- HTTP/WebSocket traffic is unencrypted: use a trusted LAN.

The old 0.1.6 Fedora-built AppImage fails to load on Ubuntu 22.04 due to newer
glibc symbol requirements. The portable 0.1.7 artifact passes on that same base.
