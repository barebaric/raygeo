//! Web Serial transport for `wasm32-unknown-unknown`.
//!
//! Enabled by the `web-serial` feature. The transport wraps a
//! `navigator.serial` port and implements the crate's [`Transport`]
//! trait, so serial protocol sessions can run in a browser by using
//! [`WebSerialTransport::as_transport_kind`] as their
//! `TransportKind::Custom`.
//!
//! Web Serial is only available in a secure context in
//! Chromium-based browsers. [`WebSerialTransport::request_port`]
//! needs a transient user activation (call it from a click handler);
//! [`WebSerialTransport::first_granted_port`] reuses a port the user
//! has already granted and needs no gesture.
//!
//! The JavaScript API is called through `js-sys` reflection rather
//! than `web-sys`, whose Web Serial bindings currently require the
//! `--cfg=web_sys_unstable_apis` build flag. This keeps enabling the
//! feature a plain Cargo feature switch.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use js_sys::{Array, Function, Object, Promise, Reflect, Uint8Array};
use tokio::sync::{mpsc, oneshot};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{spawn_local, JsFuture};

use super::{Transport, TransportError, TransportKind};

/// USB identity exposed by `SerialPort.getInfo()`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WebSerialInfo {
    /// The USB vendor ID, when the port is a USB device.
    pub usb_vendor_id: Option<u16>,
    /// The USB product ID, when the port is a USB device.
    pub usb_product_id: Option<u16>,
}

enum Command {
    Open {
        tx: mpsc::UnboundedSender<Vec<u8>>,
        reply: oneshot::Sender<Result<(), TransportError>>,
    },
    Write {
        data: Vec<u8>,
        reply: oneshot::Sender<Result<(), TransportError>>,
    },
    Close {
        reply: oneshot::Sender<()>,
    },
}

/// A byte transport over the browser's Web Serial API.
///
/// The JavaScript port is owned by a local command task; this type
/// only holds channels and plain data, so it stays `Send + Sync` and
/// can be passed to the session constructors.
pub struct WebSerialTransport {
    commands: mpsc::UnboundedSender<Command>,
    open: Arc<AtomicBool>,
    info: WebSerialInfo,
    uri: String,
}

impl WebSerialTransport {
    /// Wrap a `SerialPort` obtained from `navigator.serial`.
    ///
    /// *port* is kept on the JavaScript side; this constructor only
    /// captures its USB identity and starts the command task.
    pub fn from_port(port: JsValue, baud_rate: u32) -> Arc<Self> {
        let info = port_info(&port);
        let uri = match (info.usb_vendor_id, info.usb_product_id) {
            (Some(vendor), Some(product)) => {
                format!("webserial://{vendor:04x}:{product:04x}")
            }
            _ => "webserial://port".to_string(),
        };
        let (commands, command_rx) = mpsc::unbounded_channel();
        let open = Arc::new(AtomicBool::new(false));
        spawn_local(run_port(port, baud_rate, command_rx, open.clone()));
        Arc::new(Self {
            commands,
            open,
            info,
            uri,
        })
    }

    /// Ask the user to pick a serial port.
    ///
    /// Web Serial requires a transient user activation for
    /// `navigator.serial.requestPort()`, so call this directly from a
    /// click handler rather than after an unrelated `await`.
    pub async fn request_port(
        baud_rate: u32,
    ) -> Result<Arc<Self>, TransportError> {
        let serial = navigator_serial()?;
        let promise = call0(&serial, "requestPort")?;
        let port = await_promise(promise, "requesting a serial port").await?;
        Ok(Self::from_port(port, baud_rate))
    }

    /// Reuse the first port the user has already granted.
    ///
    /// Returns `None` when no port has been granted yet. Unlike
    /// [`request_port`](Self::request_port), this needs no user
    /// gesture.
    pub async fn first_granted_port(
        baud_rate: u32,
    ) -> Result<Option<Arc<Self>>, TransportError> {
        let serial = navigator_serial()?;
        let promise = call0(&serial, "getPorts")?;
        let ports = await_promise(promise, "listing serial ports").await?;
        let ports = ports.dyn_into::<Array>().map_err(|_| {
            TransportError::Connection(
                "getPorts() did not return an array".to_string(),
            )
        })?;
        if ports.length() == 0 {
            return Ok(None);
        }
        let port: JsValue = ports.get(0);
        if port.is_null() || port.is_undefined() {
            return Ok(None);
        }
        Ok(Some(Self::from_port(port, baud_rate)))
    }

    /// The USB identity captured when the transport was created.
    pub fn info(&self) -> &WebSerialInfo {
        &self.info
    }

    /// Wrap this transport for a session constructor.
    pub fn as_transport_kind(self: &Arc<Self>) -> TransportKind {
        TransportKind::Custom(self.clone())
    }
}

#[async_trait]
impl Transport for WebSerialTransport {
    async fn open(
        &self,
        tx: mpsc::UnboundedSender<Vec<u8>>,
    ) -> Result<(), TransportError> {
        let (reply, reply_rx) = oneshot::channel();
        self.commands
            .send(Command::Open { tx, reply })
            .map_err(|_| TransportError::NotConnected)?;
        reply_rx.await.map_err(|_| TransportError::NotConnected)?
    }

    async fn close(&self) {
        let (reply, reply_rx) = oneshot::channel();
        if self.commands.send(Command::Close { reply }).is_ok() {
            let _ = reply_rx.await;
        }
        self.open.store(false, Ordering::SeqCst);
    }

    async fn write(&self, data: &[u8]) -> Result<(), TransportError> {
        let (reply, reply_rx) = oneshot::channel();
        self.commands
            .send(Command::Write {
                data: data.to_vec(),
                reply,
            })
            .map_err(|_| TransportError::NotConnected)?;
        reply_rx.await.map_err(|_| TransportError::NotConnected)?
    }

    fn is_open(&self) -> bool {
        self.open.load(Ordering::SeqCst)
    }

    fn resource_uri(&self) -> Option<String> {
        Some(self.uri.clone())
    }
}

async fn run_port(
    port: JsValue,
    baud_rate: u32,
    mut commands: mpsc::UnboundedReceiver<Command>,
    open: Arc<AtomicBool>,
) {
    let mut reader: Option<JsValue> = None;
    while let Some(command) = commands.recv().await {
        match command {
            Command::Open { tx, reply } => {
                if open.load(Ordering::SeqCst) {
                    let error = TransportError::Connection(
                        "serial port is already open".to_string(),
                    );
                    let _ = reply.send(Err(error));
                    continue;
                }
                match open_port(&port, baud_rate).await {
                    Ok(new_reader) => {
                        reader = Some(new_reader.clone());
                        open.store(true, Ordering::SeqCst);
                        let _ = reply.send(Ok(()));
                        spawn_local(read_loop(
                            new_reader,
                            port.clone(),
                            tx,
                            open.clone(),
                        ));
                    }
                    Err(error) => {
                        close_raw(&port).await;
                        let _ = reply.send(Err(error));
                    }
                }
            }
            Command::Write { data, reply } => {
                if !open.load(Ordering::SeqCst) {
                    let _ = reply.send(Err(TransportError::NotConnected));
                    continue;
                }
                let result = write_bytes(&port, &data).await;
                if result.is_err() {
                    open.store(false, Ordering::SeqCst);
                }
                let _ = reply.send(result);
            }
            Command::Close { reply } => {
                close_port(&port, reader.take()).await;
                open.store(false, Ordering::SeqCst);
                let _ = reply.send(());
            }
        }
    }
    close_port(&port, reader.take()).await;
    open.store(false, Ordering::SeqCst);
}

async fn open_port(
    port: &JsValue,
    baud_rate: u32,
) -> Result<JsValue, TransportError> {
    let options = Object::new();
    Reflect::set(
        options.as_ref(),
        &JsValue::from_str("baudRate"),
        &JsValue::from(baud_rate),
    )
    .map_err(|error| js_error("setting baudRate", error))?;

    let promise = call1(port, "open", options.as_ref())?;
    await_promise(promise, "opening the serial port").await?;

    let readable = get_property(port, "readable")?;
    if readable.is_null() || readable.is_undefined() {
        return Err(TransportError::Connection(
            "serial port has no readable stream".to_string(),
        ));
    }
    let writable = get_property(port, "writable")?;
    if writable.is_null() || writable.is_undefined() {
        return Err(TransportError::Connection(
            "serial port has no writable stream".to_string(),
        ));
    }

    let reader = call0(&readable, "getReader")?;
    if reader.is_null() || reader.is_undefined() {
        return Err(TransportError::Connection(
            "serial port returned no reader".to_string(),
        ));
    }
    Ok(reader)
}

async fn read_loop(
    reader: JsValue,
    port: JsValue,
    tx: mpsc::UnboundedSender<Vec<u8>>,
    open: Arc<AtomicBool>,
) {
    loop {
        let chunk = match call0(&reader, "read") {
            Ok(promise) => {
                match await_promise(promise, "reading from the serial port")
                    .await
                {
                    Ok(chunk) => chunk,
                    Err(error) => {
                        log_crate::warn!("Web Serial read failed: {error}");
                        break;
                    }
                }
            }
            Err(error) => {
                log_crate::warn!("Web Serial read failed: {error}");
                break;
            }
        };

        let done = get_property(&chunk, "done")
            .ok()
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        if done {
            break;
        }

        let value = match get_property(&chunk, "value") {
            Ok(value) if !value.is_null() && !value.is_undefined() => value,
            _ => continue,
        };
        let bytes = match value.dyn_into::<Uint8Array>() {
            Ok(array) => {
                let mut bytes = vec![0u8; array.length() as usize];
                array.copy_to(&mut bytes);
                bytes
            }
            Err(_) => {
                log_crate::warn!("Web Serial delivered a non-Uint8Array chunk");
                break;
            }
        };
        if !bytes.is_empty() && tx.send(bytes).is_err() {
            break;
        }
    }

    open.store(false, Ordering::SeqCst);
    let _ = call0(&reader, "releaseLock");
    close_raw(&port).await;
}

async fn write_bytes(
    port: &JsValue,
    data: &[u8],
) -> Result<(), TransportError> {
    let writable = get_property(port, "writable")?;
    if writable.is_null() || writable.is_undefined() {
        return Err(TransportError::NotConnected);
    }
    let writer = call0(&writable, "getWriter")?;
    if writer.is_null() || writer.is_undefined() {
        return Err(TransportError::NotConnected);
    }

    let chunk = Uint8Array::from(data);
    let result = match call1(&writer, "write", chunk.as_ref()) {
        Ok(promise) => await_promise(promise, "writing to the serial port")
            .await
            .map(|_| ()),
        Err(error) => Err(error),
    };
    let _ = call0(&writer, "releaseLock");
    result
}

async fn close_port(port: &JsValue, reader: Option<JsValue>) {
    if let Some(reader) = reader {
        if let Ok(promise) = call0(&reader, "cancel") {
            let _ = await_promise(promise, "canceling the serial reader").await;
        }
        let _ = call0(&reader, "releaseLock");
    }
    close_raw(port).await;
}

async fn close_raw(port: &JsValue) {
    if let Ok(promise) = call0(port, "close") {
        let _ = await_promise(promise, "closing the serial port").await;
    }
}

fn navigator_serial() -> Result<JsValue, TransportError> {
    let global = js_sys::global();
    let navigator = get_property(global.as_ref(), "navigator")?;
    if navigator.is_null() || navigator.is_undefined() {
        return Err(TransportError::Connection(
            "no navigator object; Web Serial requires a browser".to_string(),
        ));
    }
    let serial = get_property(&navigator, "serial")?;
    if serial.is_null() || serial.is_undefined() {
        return Err(TransportError::Connection(
            "Web Serial is unavailable; use a secure context in a \
             Chromium-based browser"
                .to_string(),
        ));
    }
    Ok(serial)
}

fn port_info(port: &JsValue) -> WebSerialInfo {
    let Ok(raw) = call0(port, "getInfo") else {
        return WebSerialInfo::default();
    };
    WebSerialInfo {
        usb_vendor_id: number_property(&raw, "usbVendorId"),
        usb_product_id: number_property(&raw, "usbProductId"),
    }
}

fn number_property(target: &JsValue, name: &str) -> Option<u16> {
    get_property(target, name)
        .ok()
        .and_then(|value| value.as_f64())
        .and_then(|value| u16::try_from(value as i64).ok())
}

fn get_property(
    target: &JsValue,
    name: &str,
) -> Result<JsValue, TransportError> {
    Reflect::get(target, &JsValue::from_str(name))
        .map_err(|error| js_error(&format!("reading {name}"), error))
}

fn call0(target: &JsValue, method: &str) -> Result<JsValue, TransportError> {
    let function = get_property(target, method)?
        .dyn_into::<Function>()
        .map_err(|_| {
            TransportError::Connection(format!("{method} is not a function"))
        })?;
    let result: Result<JsValue, JsValue> = function.call0(target);
    result.map_err(|error| js_error(method, error))
}

fn call1(
    target: &JsValue,
    method: &str,
    argument: &JsValue,
) -> Result<JsValue, TransportError> {
    let function = get_property(target, method)?
        .dyn_into::<Function>()
        .map_err(|_| {
            TransportError::Connection(format!("{method} is not a function"))
        })?;
    let result: Result<JsValue, JsValue> = function.call1(target, argument);
    result.map_err(|error| js_error(method, error))
}

async fn await_promise(
    value: JsValue,
    context: &str,
) -> Result<JsValue, TransportError> {
    if !value.is_instance_of::<Promise>() {
        return Err(TransportError::Connection(format!(
            "{context}: expected a Promise"
        )));
    }
    let promise: Promise = value.unchecked_into();
    JsFuture::from(promise)
        .await
        .map_err(|error| js_error(context, error))
}

fn js_error(context: &str, error: JsValue) -> TransportError {
    let detail = error
        .dyn_ref::<js_sys::Error>()
        .map(|error| String::from(error.message()))
        .or_else(|| error.as_string())
        .or_else(|| {
            Reflect::get(&error, &JsValue::from_str("message"))
                .ok()
                .and_then(|message| message.as_string())
        })
        .unwrap_or_else(|| format!("{error:?}"));
    TransportError::Connection(format!("{context}: {detail}"))
}
