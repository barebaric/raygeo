# WebAssembly support

`raydriver` compiles for `wasm32-unknown-unknown` and — unlike a
simple "strip everything" port — keeps everything a browser can
plausibly drive: the session logic, the HTTP and WebSocket clients,
the telnet transport and the `esp3d`/`octoprint`/`xtool` sessions all
compile **and run** on wasm. Verify with `make check-wasm`.

```sh
rustup target add wasm32-unknown-unknown
cargo check --target wasm32-unknown-unknown --no-default-features
```

## What runs on wasm, and how

Everything device-facing dials through one seam,
[`core::connector::dial`](../src/core/connector.rs):

```rust
// The embedding application installs a connector at startup:
core::connector::set_stream_connector(Some(Arc::new(MyConnector)));

pub trait StreamConnector: Send + Sync {
    /// Produce a byte stream connected to (host, port), with raw
    /// TCP semantics: bytes in both directions are forwarded
    /// unchanged.
    fn connect(&self, host: &str, port: u16)
        -> Pin<Box<dyn Future<Output = io::Result<DynStream>> + Send + '_>>;
}
```

- **Native hosts** (with the `net` feature): the default is a plain
  `TcpStream`; no connector needed. Behavior is unchanged.
- **wasm**: without a connector, every dial fails with a descriptive
  error; with one, the *same* session code runs against
  browser-provided streams.

Concrete browser paths:

| Device path | How it reaches wasm |
|---|---|
| USB-serial GRBL controllers | **Web Serial API** — the embedder implements [`Transport`](../src/core/transport/mod.rs) (byte streams from `navigator.serial`) and passes it as `TransportKind::Custom` to a `GrblSession`/`SmoothieSession`/`MarlinSession`. |
| Network devices (ESP3D, OctoPrint, xTool D-series) | A **WebSocket-to-TCP bridge** (e.g. `websockify`) — the browser opens a WebSocket to the bridge, the bridge dials the device. The bridge's byte-pipe semantics are exactly what [`StreamConnector`] provides; the HTTP requests and the WS handshakes then flow over it unchanged. |
| mTLS/TLS device APIs | TLS terminates in the bridge (or in the browser for `wss://` endpoints); `ring`/rustls are never needed on wasm. |

## What is feature-gated, and why

The features exist for what a browser *physically cannot do* — not
to disable functionality:

| Feature | Gates | Why it cannot run on wasm |
|---|---|---|
| `net` | TCP-based discovery (`discovery::{daemon,probe,subnet}`), the remote client | raw TCP/UDP sockets; a browser has none (the *device-facing* dialing is deliberately **not** gated — it uses the connector seam) |
| `mdns` | mDNS announce/browse | UDP multicast + OS threads |
| `serial` | the tokio-serial transport impl | OS file descriptors (Web Serial replaces it — see above; port *enumeration* via `serialport` stays compiled and degrades gracefully) |
| `remote-daemon` | the session-host daemon | TCP listener, TLS certs from the filesystem |
| `m2` | the xTool M2 session | its tokio-tungstenite stack has no wasm build |
| `tls-native-roots` / `tls-webpki-roots` | the M2 client's TLS trust store (exactly one is required by `m2`) | no OS trust store on wasm; `m2` is off there |

`default = ["net", "mdns", "serial", "remote-daemon", "m2",
"tls-native-roots"]`, so native consumers see no change; a wasm build
is `--no-default-features`.

`m2` requires exactly one TLS root provider.  `tls-native-roots`
(the default) uses the OS trust store, which follows the
administrator's policy and keeps `webpki-roots` out of the dependency
graph — distributions that cannot ship it (e.g. Debian) build with
`--no-default-features --features tls-native-roots` (or `python`).
`tls-webpki-roots` bundles the Mozilla roots for self-contained
builds.  Both features imply `m2`; enabling `m2` without a provider
is a compile-time error.

## Embedder runtime recipe

- **Async runtime**: tokio with `rt`, `macros`, `time`, `sync`,
  `io-util` (exactly what the crate requests) on a current-thread
  runtime integrated with `wasm-bindgen-futures` (e.g. via
  `tokio_with_wasm::tokio` attribute macros, or a manual pump). The
  crate never spawns OS threads on the wasm path; blocking calls are
  either cfg'd away or behind platform-gated features.
- **Randomness**: `uuid`/`getrandom` are not compiled on wasm (the
  features that use them are gated off), so no
  `getrandom_backend` cfg is required. If you enable `m2` on a
  wasm-like target, configure
  `RUSTFLAGS='--cfg getrandom_backend="wasm_js"'`.
- **JS glue**: the crate is plain Rust (no wasm-bindgen dependency);
  the embedding crate owns the `wasm-bindgen` exports and implements
  `Transport` / `StreamConnector` over `web-sys` APIs.
