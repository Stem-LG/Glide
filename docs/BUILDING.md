# Building Glide

## Native build

Use Linux, a current Rust toolchain, GTK 4.12 or newer, and libadwaita 1.5 or newer.
The release toolchain is Rust 1.97.1. On Fedora:

```sh
sudo dnf install gcc pkgconf-pkg-config gtk4-devel libadwaita-devel python3-aiohttp
cargo build --locked --release
cargo test --locked
./target/release/glide
```

Run the app as your normal user. See the [input setup guide](../data/setup/README.md)
if `/dev/uinput` is unavailable.

## Portable AppImage (0.1.7 and later)

The portable build targets **x86_64 Linux with glibc 2.35 or newer**, using Ubuntu
22.04 as its compiler/runtime baseline. It builds pinned GTK, libadwaita, GLib,
and supporting UI libraries from source. System libc and graphics drivers are
left on the host. This is not a promise of compatibility with every Linux system:
Alpine/musl, older glibc versions, and ARM require different builds.

The packaging step scans **every bundled ELF file** and fails if any requires a
GLIBC symbol newer than 2.35. Native source versions and SHA-256 hashes live in
`scripts/portable/sources.json`. The AppImage runtime and its sources are pinned
separately in `scripts/portable/runtime-sources.json`.

From a checked-out source repository:

```sh
docker build -f scripts/portable/Containerfile -t glide-portable .
docker run -d --name glide-portable-build -v "$PWD:/work" glide-portable sleep infinity
docker exec glide-portable-build bash scripts/portable/build.sh
```

The image includes the Rust toolchain; no host toolchain is needed. Downloads and
builds happen inside the container. Build intermediates are in `build/portable`
and `target/portable`, and outputs are in `dist/portable`.

The existing Fedora packaging scripts remain available for reproducing v0.1.6;
they are not the portable release path.

### Corresponding sources and checksums

The container above is retained so the exact installed Ubuntu package versions
can be mapped to their source packages. Collect sources before removing it:

```sh
docker exec glide-portable-build bash -c \
  'git config --global --add safe.directory /work && python3 scripts/portable/collect-sources.py && python3 scripts/portable/archive-sources.py'
```

Stop the builder with `docker stop glide-portable-build` when finished. Source
collection requires network access and the exact Ubuntu source versions to remain
available in its configured apt repositories.
The archive script includes Git-tracked application files; commit or stage new
release files before running it.

Release outputs:

- `Glide-<version>-x86_64.AppImage`
- `Glide-<version>-sources.tar.gz`
- `native-packages.tsv`, `rust-crates.tsv`, `build-info.json`
- `SHA256SUMS`

The upstream runtime download uses a moving upstream URL but a fixed SHA-256.
If upstream replaces it, the build deliberately fails rather than silently
changing runtime versions. Supply the matching runtime from a previous Glide
source bundle, or review and update the runtime source/binary pins together.

### Rebuilding from the source archive

Extract `Glide-<version>-sources.tar.gz`. `glide/` contains the application source,
`vendor/` contains the locked Rust crates, `native/` contains Ubuntu source
packages (upstream archives, Debian patches and build instructions), `upstream/`
contains the additional pinned UI library tarballs, and `appimage-runtime/`
contains the runtime and its libfuse/squashfuse source archives.

To build the application with the supplied Rust dependencies:

```sh
cd glide
mkdir -p .cargo
cp ../vendor-config.toml .cargo/config.toml
ln -s ../vendor vendor
cargo build --locked --offline --release
```

The Rust toolchain and native development libraries must already be installed;
`--offline` only controls Cargo dependencies.

To reproduce the portable native stack, copy `upstream/*` into
`glide/build/portable/sources/` before running the container build recipe. Rebuild
Ubuntu source packages with `dpkg-source -x <package>.dsc` and
`dpkg-buildpackage` in an Ubuntu 22.04 environment with the source package's build
dependencies installed. Package mappings are recorded in `native-packages.tsv`.
To reuse the exact AppImage runtime, copy `runtime-x86_64` into `glide/dist/` and
`appimage-runtime/` into `glide/dist/source-bundle/`. To modify/relink the runtime,
extract its upstream source archive and follow the included build recipe/patches.

Build timestamps and environment changes may affect output hashes. These are
rebuild instructions, not a claim of bit-for-bit reproducibility.

## Compatibility tests

Run `scripts/portable/test-container.sh /absolute/path/to/Glide.AppImage` inside a
clean distro container with `/work` pointing at this repository. It extracts the
actual AppImage and verifies its version, pairing, settings persistence, GTK
window startup under Xvfb, and the GUI's HTTP health endpoint. It does not access
host input devices. See `.github/workflows/portable.yml` for the runtime-only
packages and Ubuntu/Debian/Fedora matrix.

These tests establish loading and application behavior on those userspaces.
They do not emulate each distribution's full GNOME session or prove native
gestures on every desktop. The native gesture tests in `tests/` separately test a
freshly created, exclusively grabbed virtual device on the host.
