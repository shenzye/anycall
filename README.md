# anycall

A lightweight RPC framework for Rust with a focus on readability and developer ergonomics.

## Workspace crates

- `anycall`: core traits, codecs, and generated client/server glue.
- `anycall-macro`: proc-macros used by `anycall` (`#[anycall::service]`).
- `anycall-protocol`: transport adapters, enabled via Cargo features.

## Install

```toml
[dependencies]
anycall = { version = "0.1" }
anycall-protocol = { version = "0.1", features = ["axum", "reqwest"] }
```

Protocol features (all optional, none enabled by default):

- `axum` — HTTP server via `AxumBodyHandler`
- `reqwest` — HTTP client via `ReqwestPost`
- `iroh` — QUIC peer transport via `IrohHandler` / `IrohConnect` (native and `wasm32-unknown-unknown`)
- `channel` — in-process sync channel via `new_channel_pair`

## Basic usage

```rust
#[anycall::service]
pub trait Counter {
    fn sum(&self, a: i32, b: i32) -> i32;
    async fn sum_async(&self, a: i32, b: i32) -> i32;
}
```

Use the generated `*Client` and `*Service` types with a transport adapter such as:

- `anycall_protocol::axum::AxumBodyHandler` (feature `axum`)
- `anycall_protocol::reqwest::ReqwestPost` (feature `reqwest`)
- `anycall_protocol::iroh::IrohHandler` / `anycall_protocol::iroh::IrohConnect` (feature `iroh`)
- `anycall_protocol::channel::new_channel_pair` (feature `channel`)

For a runnable end-to-end example, see `test/src/counter.rs`.

## License

MIT
