# anycall

A lightweight RPC framework for Rust with a focus on readability and developer ergonomics.

## Workspace crates

- `anycall`: core traits, codecs, and generated client/server glue.
- `anycall-macro`: proc-macros used by `anycall` (`#[anycall::service]`).
- `anycall-protocol`: transport adapters for common HTTP stacks.

## Install

```toml
[dependencies]
anycall = { version = "0.1" }
anycall-protocol = { version = "0.1" }
```

## Basic usage

```rust
#[anycall::service]
pub trait Counter {
    fn sum(&self, a: i32, b: i32) -> i32;
    async fn sum_async(&self, a: i32, b: i32) -> i32;
}
```

Use the generated `*Client` and `*Service` types with a transport adapter such as:

- `anycall_protocol::axum::AxumBodyHandler`
- `anycall_protocol::reqwest::ReqwestPost`

For a runnable end-to-end example, see `test/src/counter.rs`.

## License

MIT
