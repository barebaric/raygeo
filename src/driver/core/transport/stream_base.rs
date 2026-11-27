//! Shared async plumbing for transports that are a single byte
//! stream split into reader and writer halves (serial, telnet).

use std::marker::Unpin;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::io::{
    AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadHalf, WriteHalf,
};
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use super::TransportError;

pub(crate) const READ_BUFFER_SIZE: usize = 1024;

/// The open/writer/reader state shared by stream transports.
pub(crate) struct StreamTransportBase<S> {
    open: AtomicBool,
    writer: Mutex<Option<WriteHalf<S>>>,
    reader_task: Mutex<Option<JoinHandle<()>>>,
}

impl<S> StreamTransportBase<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    pub(crate) fn new() -> Self {
        Self {
            open: AtomicBool::new(false),
            writer: Mutex::new(None),
            reader_task: Mutex::new(None),
        }
    }

    /// Split *stream*, spawn the receive loop feeding `tx`, and mark
    /// the transport open.
    pub(crate) async fn start(
        &self,
        stream: S,
        log_label: &'static str,
        tx: mpsc::UnboundedSender<Vec<u8>>,
    ) {
        let (read_half, write_half) = tokio::io::split(stream);
        *self.writer.lock().await = Some(write_half);
        let task = tokio::spawn(reader_loop(read_half, log_label, tx));
        *self.reader_task.lock().await = Some(task);
        self.open.store(true, Ordering::SeqCst);
    }

    pub(crate) async fn close(&self) {
        self.open.store(false, Ordering::SeqCst);
        if let Some(task) = self.reader_task.lock().await.take() {
            task.abort();
        }
        *self.writer.lock().await = None;
    }

    pub(crate) async fn write(
        &self,
        data: &[u8],
    ) -> Result<(), TransportError> {
        let mut writer_guard = self.writer.lock().await;
        let Some(writer) = writer_guard.as_mut() else {
            return Err(TransportError::NotConnected);
        };
        match writer.write_all(data).await {
            Ok(()) => {
                let _ = writer.flush().await;
                Ok(())
            }
            Err(e) => {
                self.open.store(false, Ordering::SeqCst);
                Err(TransportError::Connection(e.to_string()))
            }
        }
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open.load(Ordering::SeqCst)
    }
}

async fn reader_loop<S: AsyncRead + Unpin>(
    mut read_half: ReadHalf<S>,
    log_label: &'static str,
    tx: mpsc::UnboundedSender<Vec<u8>>,
) {
    let mut buf = vec![0u8; READ_BUFFER_SIZE];
    loop {
        match read_half.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => {
                if tx.send(buf[..n].to_vec()).is_err() {
                    break;
                }
            }
            Err(e) => {
                log_crate::debug!("{log_label} read error: {e}");
                break;
            }
        }
    }
}
