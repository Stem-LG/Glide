# Contributing to Glide

Please open an issue before a large change so we can agree on scope. Small fixes,
clear reproduction steps, and reports from other Linux desktops are welcome.

Use a current stable Rust toolchain and GTK4/libadwaita development packages. Run
`cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`, and
`cargo test --locked`. The README lists network and native-input regression tests.

Use `GLIDE_CONFIG_DIR` with a temporary directory and `--preview` for web development.
Do not test synthetic gestures on a real desktop session without isolating/grabbing
the test device. Keep tests specific to the behavior changed.

A pull request should explain the problem, resulting behavior, and validation.
For UI changes, include a screenshot. Do not commit pairing credentials, personal
configuration, build outputs, or local input traces.

Contributions are made under the project's GPL-3.0-or-later license. Third-party
code must include its provenance and compatible license notices.
