# Building Glide

## Native build

Use Linux, Rust 1.97.1 (the release toolchain), GTK 4.12 or newer, and libadwaita
1.5 or newer. On Fedora:

```sh
sudo dnf install gcc pkgconf-pkg-config gtk4-devel libadwaita-devel python3-aiohttp
cargo build --locked --release
cargo test --locked
./target/release/glide
```

The release was built on Fedora 44, with GTK 4.22.5 and libadwaita 1.9.4. Building
on an older compatible distro is preferable when targeting its libc baseline.
The current AppImage does not promise compatibility with older distributions.

## Container and AppImage

`scripts/Containerfile` provides Fedora's build dependencies. Install Rust with
rustup inside it, or mount an existing toolchain and set `CARGO_HOME`,
`RUSTUP_HOME`, and `PATH`. Do not run the GUI as root. For a fresh build image:

```sh
docker build -f scripts/Containerfile -t glide-build .
docker run --rm -it -v "$PWD:/work" glide-build bash
# Inside the container, install/select Rust 1.97.1, then:
bash scripts/build.sh
python3 scripts/collect-sources.py
python3 scripts/rust-notices.py
# Place the pinned runtime executable at dist/runtime-x86_64 (see below).
python3 scripts/package.py
```

The container needs network access to download dependencies. Source collection
uses DNF source repositories and Fedora Koji for exact superseded builds.
Packaging requires `mksquashfs`, the vendored dependency notices, inventories,
and the pinned type-2 runtime. The runtime SHA-256 is checked before packaging.
The resulting AppImage replaces its previous path atomically, so packaging does
not truncate an image currently mounted by a running instance.

## Release source bundle

Extract `Glide-0.1.6-sources.tar.gz`. Its `glide/` directory is the application
source. To build using the supplied Cargo sources:

```sh
cd glide
mkdir -p .cargo
cp ../vendor-config.toml .cargo/config.toml
# The generated config uses a project-relative directory named vendor.
ln -s ../vendor vendor
cargo build --locked --offline --release
```

Native development libraries and a Rust toolchain must already be installed;
`--offline` only makes Cargo's dependency resolution offline.

`native/` contains exact Fedora source RPMs. Each includes source archives,
patches and a `.spec` describing its build and configuration. Rebuild using
Fedora's `rpmbuild --rebuild` or `mock` with the appropriate Fedora 44 build
requirements. See `native-packages.tsv` for the binary/source mapping.

`appimage-runtime/` contains the upstream runtime source archive at commit
`fedfad5`, plus libfuse 3.15.0 and squashfuse 0.5.2 archives. Extract the runtime
source and follow its README/build scripts; its Docker recipe specifies Alpine
and the supporting development packages, and includes the libfuse patch. These
sources permit rebuilding/relinking the runtime with modified libraries.
For the original packaging runtime, copy the supplied `runtime-x86_64` into
`glide/dist/`; it is checksum-pinned in `scripts/package.py`.

To repackage from this bundle, make `glide/dist/source-bundle` point to the
extracted bundle root, copy its inventories to `glide/dist`, run
`scripts/rust-notices.py`, then `scripts/package.py` inside the Fedora build
environment. Packaging modified native dependencies also requires regenerating
and distributing their corresponding source inventory/archive.

Build timestamps, toolchain/environment updates, and SquashFS metadata can affect
binary hashes. These instructions provide rebuildable source, not a claim of
bit-for-bit reproducibility.
