# Acheron

The Rust CLI and TUI for this repository's workspace deployment architecture.
See the [repository README](../README.md) for host testing, installation,
manifest syntax, data locations, lifecycle semantics, and release automation.

```sh
cargo build --release --locked
cargo run --locked -- --demo
cargo test --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
```

The library separates configuration resolution, the Distrobox/Stow backend,
deployment planning and operations, local receipts, and environment detection.
CLI and TUI call the same engine. Deployment tests use an in-memory container
backend and temporary real directories; Stow integration exercises conflicts
and repeat configuration without modifying the host home.
