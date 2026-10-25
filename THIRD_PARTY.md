# Third-party components

Glide's own code is GPL-3.0-or-later. Dependencies and bundled assets retain their
original licenses; inclusion does not relicense them.

The release's `rust-crates.tsv` records all locked Cargo dependencies (including
platform-specific and development dependencies). `native-packages.tsv` maps the
Ubuntu-provided bundled libraries and icon themes to their exact Ubuntu source
packages. Additional UI libraries are built from the checksum-pinned upstream
sources listed in `scripts/portable/sources.json` and `build-info.json`.

The AppImage contains original native package notices under `usr/share/licenses`,
Rust crate notices under `usr/share/licenses/rust-crates`, and Rust standard-library
notices under `usr/share/licenses/rust-stdlib`. Full crate sources and their
original notices are also in the accompanying source bundle.

The AppImage type-2 runtime is from AppImage/type2-runtime commit `fedfad5`, with
SHA-256 `617aa77314d57c8b8e4fae9c3f0b35af7b30d5529cb69e98f64f92510ed98a46`.
It uses libfuse 3.15.0 and squashfuse 0.5.2; their source archives and the runtime
build recipe/patches accompany the release. Runtime and dependency notices are
in `docs/licenses/runtime`. Other runtime dependencies include musl, zstd, zlib,
and mimalloc; consult the upstream Alpine build recipe for how these are linked.

## Corresponding sources

Download `Glide-0.1.8-sources.tar.gz` beside the AppImage. It includes:

- Glide's release source and build/packaging scripts;
- all Rust crates from `Cargo.lock`, with a Cargo vendor configuration;
- exact Ubuntu source packages for bundled distro libraries, including upstream
  sources, Debian patches, and build instructions;
- pinned upstream tarballs for the newer UI stack and its build recipe;
- AppImage runtime source, build recipe and patches, libfuse and squashfuse sources;
- the pinned runtime executable used to assemble this AppImage, and inventories.

See [docs/BUILDING.md](docs/BUILDING.md) for rebuilding. System components such as
host glibc and GPU drivers are not bundled. The dependency archive is provided
alongside the binary rather than relying on an offer to provide sources later.
