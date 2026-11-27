# WebAssembly support

raygeo compiles for `wasm32-unknown-unknown` with
`--no-default-features`. The core layers are pure Rust and free of
host-specific requirements; the driver layer keeps the parts a browser
can plausibly drive.

```sh
rustup target add wasm32-unknown-unknown
cargo check --lib --target wasm32-unknown-unknown --no-default-features
```

This target is for Rust embedders. The Python extension is not built
for wasm. The optional `browser` feature adds the browser
integrations; it currently enables the Web Serial transport
(`web-serial`).

## What runs on wasm

The platform-independent core compiles unchanged:

- `geo`, `sketch`, `ops` (including `ops.convert`), `cnc`, `pipeline`,
  `formats`, `barcode`, `template`, `fstring` and `compressed_array`.
- Driver sessions and their protocol stacks: the line protocol logic,
  HTTP and WebSocket clients, the telnet transport, and the
  `esp3d`/`octoprint`/`xtool_d1` sessions.
- Device I/O dials through the connector seam below; the embedder
  supplies browser-backed streams.

Host-only functionality is feature-gated rather than stripped: mDNS,
TCP discovery, OS serial ports, the remote daemon and the xTool M2
session.

## Device I/O on wasm

Everything device-facing dials through one seam,
[`driver::core::connector::dial`](../src/driver/core/connector.rs):

```rust
// The embedding application installs a connector at startup:
driver::core::connector::set_stream_connector(Some(Arc::new(MyConnector)));

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
| USB-serial GRBL controllers | **Web Serial API** — enable the `browser` feature (`web-serial` alone is also available) and pass its `WebSerialTransport` as `TransportKind::Custom` to a `GrblSession`/`SmoothieSession`/`MarlinSession`; embedders that do not enable it can implement [`Transport`](../src/driver/core/transport/mod.rs) themselves. |
| Network devices (ESP3D, OctoPrint, xTool D-series) | A **WebSocket-to-TCP bridge** (e.g. `websockify`) — the browser opens a WebSocket to the bridge, the bridge dials the device. The bridge's byte-pipe semantics are exactly what [`StreamConnector`] provides; the HTTP requests and the WS handshakes then flow over it unchanged. |
| mTLS/TLS device APIs | TLS terminates in the bridge (or in the browser for `wss://` endpoints); `ring`/rustls are never needed on wasm. |

### Web Serial

The `browser` bundle currently enables the `web-serial` capability,
which provides
`driver::core::transport::web_serial::WebSerialTransport`, a
`Transport` over `navigator.serial`:

```rust
use raygeo::driver::core::transport::web_serial::WebSerialTransport;

// Call from a click handler: requestPort() needs a user gesture.
let transport = WebSerialTransport::request_port(115_200).await?;
let kind = transport.as_transport_kind();
// Pass `kind` to a session constructor, e.g. SmoothieConfig::new(kind).

// Reuse a previously granted port; no gesture is required.
let transport = WebSerialTransport::first_granted_port(115_200)
    .await?
    .expect("no serial port granted yet");
```

```sh
cargo check --lib --target wasm32-unknown-unknown \
    --no-default-features --features browser
```

Web Serial is available in secure contexts in Chromium-based browsers.
`request_port()` must be called from a transient user activation;
`first_granted_port()` reuses a port the user has already granted.
The transport is only compiled for `wasm32-unknown-unknown`.

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

- **Async runtime**: driver sessions use tokio with `rt`, `macros`,
  `time`, `sync` and `io-util` on a current-thread runtime integrated
  with `wasm-bindgen-futures` (e.g. via `tokio_with_wasm::tokio`
  attribute macros, or a manual pump). The geometry, ops and format
  layers are synchronous and need no runtime.
- **Randomness**: `getrandom` uses its `wasm_js` backend on wasm32
  (nesting uses `rand`), so the embedder must provide the wasm-bindgen
  JS glue. No `getrandom_backend` cfg is required.
- **JS glue**: raygeo exposes no `wasm-bindgen` API; the embedding
  crate owns the exports and implements `Transport` / `StreamConnector`
  over `web-sys` APIs. (On wasm32, `getrandom`'s `wasm_js` backend
  already pulls `wasm-bindgen` for randomness.) The optional `browser`
  feature (currently `web-serial`) adds a ready-made `Transport` over
  `navigator.serial` for `wasm32-unknown-unknown`.
