//! Byte links and the packet handshake of the Ruida protocol.
//!
//! USB links carry raw swizzled bytes and are not acknowledged. UDP
//! and TCP links acknowledge every packet with a single byte; packets
//! that carry a ``GET_SETTING`` command are followed by one or more
//! replies. Replies are framed by the high bit of their leading
//! ``0xDA`` byte and reassembled across reads until an inter-packet
//! gap.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;

use crate::driver::core::driver::{DriverError, DriverEvents};
use crate::driver::ruida::codec;

/// The physical link a session runs over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportKind {
    Usb,
    Udp,
    Tcp,
}

impl TransportKind {
    pub fn is_udp(self) -> bool {
        self == Self::Udp
    }

    pub fn is_usb(self) -> bool {
        self == Self::Usb
    }
}

/// A byte link to the controller.
#[async_trait]
pub trait Link: Send {
    async fn write(&mut self, data: &[u8]) -> Result<(), DriverError>;
    async fn read(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<Vec<u8>>, DriverError>;
    async fn close(&mut self) {}
}

fn timed_out() -> DriverError {
    DriverError::Timeout("controller did not answer".to_string())
}

/// Wraps a link and mirrors every wire chunk into raw driver events,
/// so the protocol analyzer can record binary traffic.  Chunks stay
/// swizzled: they are exactly what crossed the wire.
pub(crate) struct TappedLink {
    link: Box<dyn Link>,
    events: Arc<dyn DriverEvents>,
}

/// Upper bound for one raw event, so a large USB job cannot create
/// an oversized event (or remote wire frame).
const RAW_EVENT_CHUNK: usize = 64 * 1024;

impl TappedLink {
    pub(crate) fn new(
        link: Box<dyn Link>,
        events: Arc<dyn DriverEvents>,
    ) -> Self {
        Self { link, events }
    }

    fn report_tx(&self, data: &[u8]) {
        for chunk in data.chunks(RAW_EVENT_CHUNK) {
            self.events.raw_tx_bytes(chunk);
        }
    }

    fn report_rx(&self, data: &[u8]) {
        for chunk in data.chunks(RAW_EVENT_CHUNK) {
            self.events.raw_rx_bytes(chunk);
        }
    }
}

#[async_trait]
impl Link for TappedLink {
    async fn write(&mut self, data: &[u8]) -> Result<(), DriverError> {
        self.link.write(data).await?;
        self.report_tx(data);
        Ok(())
    }

    async fn read(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<Vec<u8>>, DriverError> {
        let data = self.link.read(timeout).await?;
        if let Some(bytes) = data.as_deref() {
            self.report_rx(bytes);
        }
        Ok(data)
    }

    async fn close(&mut self) {
        self.link.close().await;
    }
}

/// A link plus the Ruida packet handshake.
pub struct RuidaTransport {
    link: Box<dyn Link>,
    kind: TransportKind,
    magic: u8,
    ack_table: [u8; 256],
    rx: Vec<u8>,
    pub timeout: Duration,
    pub inter_packet_timeout: Duration,
    pub replies: Vec<codec::Reply>,
}

impl RuidaTransport {
    /// Wraps a link. ``magic`` is the controller's swizzle key.
    pub fn new(link: Box<dyn Link>, kind: TransportKind, magic: u8) -> Self {
        Self {
            link,
            kind,
            magic,
            ack_table: codec::ack_magic_table(),
            rx: Vec::new(),
            timeout: Duration::from_millis(500),
            inter_packet_timeout: Duration::from_millis(50),
            replies: Vec::new(),
        }
    }

    pub fn kind(&self) -> TransportKind {
        self.kind
    }

    pub fn magic(&self) -> u8 {
        self.magic
    }

    /// Sends raw (unswizzled) command bytes, chunking them into
    /// packets and acknowledging each one.
    pub async fn send(&mut self, raw: &[u8]) -> Result<(), DriverError> {
        const CHUNK_SIZE: usize = 1024;
        for chunk in raw.chunks(CHUNK_SIZE) {
            let packet = codec::package(chunk, self.magic, self.kind.is_udp());
            self.send_packet(&packet).await?;
        }
        Ok(())
    }

    /// Sends one raw command as a single packet.
    pub async fn send_command(
        &mut self,
        raw: &[u8],
    ) -> Result<(), DriverError> {
        let packet = codec::package(raw, self.magic, self.kind.is_udp());
        self.send_packet(&packet).await
    }

    async fn send_packet(&mut self, packet: &[u8]) -> Result<(), DriverError> {
        self.link.write(packet).await?;
        if self.kind.is_usb() {
            if self.has_get_setting(packet) {
                self.collect_replies().await?;
            }
            return Ok(());
        }
        let Some(data) = self.read_unit(self.timeout).await? else {
            return Err(timed_out());
        };
        let Some(first) = data.first().copied() else {
            return Err(timed_out());
        };
        if codec::unswizzle_byte(first, self.magic) == codec::ACK {
            if data.len() > 1 {
                self.rx.extend_from_slice(&data[1..]);
            }
            if self.has_get_setting(packet) {
                self.collect_replies().await?;
            }
            return Ok(());
        }
        if data.len() == 1 {
            let magic = self.ack_table[usize::from(first)];
            if magic != 0 {
                self.magic = magic;
            }
        }
        if data.len() > 1 {
            self.rx.extend_from_slice(&data[1..]);
        }
        Err(DriverError::Connection(
            "controller rejected the packet".to_string(),
        ))
    }

    fn has_get_setting(&self, packet: &[u8]) -> bool {
        let offset = usize::from(self.kind.is_udp()) * 2;
        let marker = codec::swizzle_byte(0xDA, self.magic);
        packet[offset..].contains(&marker)
    }

    /// Reads replies until no more data arrives within the
    /// inter-packet gap.
    async fn collect_replies(&mut self) -> Result<(), DriverError> {
        let Some(mut data) = self.read_unit(self.timeout).await? else {
            return Ok(());
        };
        loop {
            match self.link.read(self.inter_packet_timeout).await {
                Ok(Some(more)) => data.extend(more),
                Ok(None) => break,
                Err(error) => return Err(error),
            }
        }
        let raw = codec::unswizzle(&data, self.magic);
        self.replies.extend(codec::unpack_replies(&raw));
        Ok(())
    }

    async fn read_unit(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<Vec<u8>>, DriverError> {
        if !self.rx.is_empty() {
            return Ok(Some(std::mem::take(&mut self.rx)));
        }
        self.link.read(timeout).await
    }

    /// Takes the replies collected so far.
    pub fn take_replies(&mut self) -> Vec<codec::Reply> {
        std::mem::take(&mut self.replies)
    }

    /// Closes the underlying link.
    pub async fn close(&mut self) {
        self.link.close().await;
        self.rx.clear();
    }
}

#[cfg(feature = "net")]
mod net {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpStream, UdpSocket};

    fn io_error(error: std::io::Error) -> DriverError {
        DriverError::Connection(error.to_string())
    }

    pub struct UdpLink {
        socket: UdpSocket,
        buffer: Vec<u8>,
    }

    impl UdpLink {
        /// Binds the host's Ruida port and connects to the controller.
        pub async fn connect(
            host: &str,
            port: u16,
        ) -> Result<Self, DriverError> {
            let probe = UdpSocket::bind("0.0.0.0:0").await.map_err(io_error)?;
            probe.connect((host, port)).await.map_err(io_error)?;
            let local = probe.local_addr().map_err(io_error)?;
            let socket = UdpSocket::bind((local.ip(), 40200))
                .await
                .map_err(io_error)?;
            socket.connect((host, port)).await.map_err(io_error)?;
            Ok(Self {
                socket,
                buffer: vec![0; 64 * 1024],
            })
        }
    }

    #[async_trait]
    impl Link for UdpLink {
        async fn write(&mut self, data: &[u8]) -> Result<(), DriverError> {
            self.socket.send(data).await.map_err(io_error)?;
            Ok(())
        }

        async fn read(
            &mut self,
            timeout: Duration,
        ) -> Result<Option<Vec<u8>>, DriverError> {
            match tokio::time::timeout(
                timeout,
                self.socket.recv(&mut self.buffer),
            )
            .await
            {
                Ok(Ok(length)) => Ok(Some(self.buffer[..length].to_vec())),
                Ok(Err(error)) => {
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock
                            | std::io::ErrorKind::ConnectionReset
                            | std::io::ErrorKind::Interrupted
                    ) {
                        Ok(None)
                    } else {
                        Err(io_error(error))
                    }
                }
                Err(_) => Ok(None),
            }
        }

        async fn close(&mut self) {}
    }

    pub struct TcpLink {
        stream: TcpStream,
        buffer: Vec<u8>,
    }

    impl TcpLink {
        /// Connects to the controller's Ruida port.
        pub async fn connect(
            host: &str,
            port: u16,
        ) -> Result<Self, DriverError> {
            let stream = tokio::time::timeout(
                Duration::from_secs(3),
                TcpStream::connect((host, port)),
            )
            .await
            .map_err(|_| timed_out())?
            .map_err(io_error)?;
            let _ = stream.set_nodelay(true);
            Ok(Self {
                stream,
                buffer: vec![0; 4096],
            })
        }
    }

    #[async_trait]
    impl Link for TcpLink {
        async fn write(&mut self, data: &[u8]) -> Result<(), DriverError> {
            self.stream.write_all(data).await.map_err(io_error)
        }

        async fn read(
            &mut self,
            timeout: Duration,
        ) -> Result<Option<Vec<u8>>, DriverError> {
            match tokio::time::timeout(
                timeout,
                self.stream.read(&mut self.buffer),
            )
            .await
            {
                Ok(Ok(0)) => Err(DriverError::Connection(
                    "controller closed the connection".to_string(),
                )),
                Ok(Ok(length)) => Ok(Some(self.buffer[..length].to_vec())),
                Ok(Err(error)) => Err(io_error(error)),
                Err(_) => Ok(None),
            }
        }

        async fn close(&mut self) {
            let _ = self.stream.shutdown().await;
        }
    }
}

#[cfg(feature = "net")]
pub use net::{TcpLink, UdpLink};

#[cfg(feature = "serial")]
mod serial {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio_serial::{SerialPortBuilderExt, SerialStream};

    fn io_error(error: std::io::Error) -> DriverError {
        DriverError::Connection(error.to_string())
    }

    pub struct SerialLink {
        port: SerialStream,
        buffer: Vec<u8>,
    }

    impl SerialLink {
        /// Opens a USB serial link at the Ruida baud rate.
        pub fn open(device: &str) -> Result<Self, DriverError> {
            let port: SerialStream = tokio_serial::new(device, 115_200)
                .open_native_async()
                .map_err(|error| DriverError::Connection(error.to_string()))?;
            Ok(Self {
                port,
                buffer: vec![0; 4096],
            })
        }
    }

    #[async_trait]
    impl Link for SerialLink {
        async fn write(&mut self, data: &[u8]) -> Result<(), DriverError> {
            self.port.write_all(data).await.map_err(io_error)
        }

        async fn read(
            &mut self,
            timeout: Duration,
        ) -> Result<Option<Vec<u8>>, DriverError> {
            match tokio::time::timeout(
                timeout,
                self.port.read(&mut self.buffer),
            )
            .await
            {
                Ok(Ok(0)) => Ok(None),
                Ok(Ok(length)) => Ok(Some(self.buffer[..length].to_vec())),
                Ok(Err(error)) => Err(io_error(error)),
                Err(_) => Ok(None),
            }
        }

        async fn close(&mut self) {}
    }
}

#[cfg(feature = "serial")]
pub use serial::SerialLink;

/// Opens a link for the given endpoint.
pub async fn open_link(
    kind: TransportKind,
    host: Option<&str>,
    device: Option<&str>,
    port: u16,
) -> Result<Box<dyn Link>, DriverError> {
    let _ = (host, device, port);
    match kind {
        #[cfg(feature = "serial")]
        TransportKind::Usb => {
            let device = device.ok_or_else(|| {
                DriverError::Device("no USB device configured".to_string())
            })?;
            Ok(Box::new(SerialLink::open(device)?))
        }
        #[cfg(not(feature = "serial"))]
        TransportKind::Usb => Err(DriverError::Unsupported(
            "serial support is not compiled in".to_string(),
        )),
        #[cfg(feature = "net")]
        TransportKind::Udp => {
            let host = host.ok_or_else(|| {
                DriverError::Device("no controller host configured".to_string())
            })?;
            Ok(Box::new(UdpLink::connect(host, port).await?))
        }
        #[cfg(not(feature = "net"))]
        TransportKind::Udp => Err(DriverError::Unsupported(
            "network support is not compiled in".to_string(),
        )),
        #[cfg(feature = "net")]
        TransportKind::Tcp => {
            let host = host.ok_or_else(|| {
                DriverError::Device("no controller host configured".to_string())
            })?;
            Ok(Box::new(TcpLink::connect(host, port).await?))
        }
        #[cfg(not(feature = "net"))]
        TransportKind::Tcp => Err(DriverError::Unsupported(
            "network support is not compiled in".to_string(),
        )),
    }
}
