//! Minimal HTTP/1.1 client for device APIs.
//!
//! Hand-rolled so the crate stays free of an HTTP client dependency:
//! the APIs this module serves are small and fixed (short GET/POST
//! replies and a multipart upload endpoint).  Requests are always
//! sent with `Connection: close` and the response is read to EOF,
//! which keeps the parser simple; both `Content-Length` and
//! `Transfer-Encoding: chunked` bodies are decoded.
//!
//! Dialing goes through [`crate::driver::core::connector::dial`]: on native
//! hosts a plain TCP connection, or the connector the embedding
//! application installed — so this module and every session built on
//! it compile and run on all targets, including wasm.  The exchange
//! itself is generic over the byte stream
//! ([`request_on_stream`]).

use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::driver::core::driver::DriverError;

/// A parsed HTTP response.
#[derive(Debug)]
pub struct HttpResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

/// Perform a GET request.
pub async fn get(
    host: &str,
    port: u16,
    path: &str,
    timeout: Duration,
) -> Result<HttpResponse, DriverError> {
    get_with_headers(host, port, path, &[], timeout).await
}

/// Perform a GET request with extra request headers.
pub async fn get_with_headers(
    host: &str,
    port: u16,
    path: &str,
    extra_headers: &[(&str, &str)],
    timeout: Duration,
) -> Result<HttpResponse, DriverError> {
    request(host, port, "GET", path, None, extra_headers, timeout).await
}

/// Perform a POST request with a plain-text body.
pub async fn post(
    host: &str,
    port: u16,
    path: &str,
    content_type: &str,
    body: &[u8],
    timeout: Duration,
) -> Result<HttpResponse, DriverError> {
    post_with_headers(host, port, path, content_type, body, &[], timeout).await
}

/// Perform a POST request with extra request headers.
#[allow(clippy::too_many_arguments)]
pub async fn post_with_headers(
    host: &str,
    port: u16,
    path: &str,
    content_type: &str,
    body: &[u8],
    extra_headers: &[(&str, &str)],
    timeout: Duration,
) -> Result<HttpResponse, DriverError> {
    request(
        host,
        port,
        "POST",
        path,
        Some((content_type, body)),
        extra_headers,
        timeout,
    )
    .await
}

/// Perform a POST request with a `multipart/form-data` body carrying
/// a single file field.
pub async fn post_multipart(
    host: &str,
    port: u16,
    path: &str,
    field: &str,
    filename: &str,
    file: &[u8],
    timeout: Duration,
) -> Result<HttpResponse, DriverError> {
    post_multipart_fields(
        host,
        port,
        path,
        field,
        filename,
        file,
        &[],
        &[],
        timeout,
    )
    .await
}

/// Perform a POST request with a `multipart/form-data` body carrying
/// a single file field plus plain value fields and extra request
/// headers.
#[allow(clippy::too_many_arguments)]
pub async fn post_multipart_fields(
    host: &str,
    port: u16,
    path: &str,
    field: &str,
    filename: &str,
    file: &[u8],
    fields: &[(&str, &str)],
    extra_headers: &[(&str, &str)],
    timeout: Duration,
) -> Result<HttpResponse, DriverError> {
    let boundary = multipart_boundary();
    let mut body = Vec::with_capacity(file.len() + 256);
    body.extend_from_slice(
        format!(
            "--{b}\r\n\
             Content-Disposition: form-data; name=\"{field}\"; \
             filename=\"{filename}\"\r\n\
             Content-Type: text/plain\r\n\
             \r\n",
            b = boundary,
            field = field,
            filename = filename,
        )
        .as_bytes(),
    );
    body.extend_from_slice(file);
    for (name, value) in fields {
        body.extend_from_slice(
            format!(
                "\r\n--{b}\r\n\
                 Content-Disposition: form-data; name=\"{name}\"\r\n\
                 \r\n{value}",
                b = boundary,
                name = name,
                value = value,
            )
            .as_bytes(),
        );
    }
    body.extend_from_slice(format!("\r\n--{b}--\r\n", b = boundary).as_bytes());
    let content_type = format!("multipart/form-data; boundary={boundary}");
    request(
        host,
        port,
        "POST",
        path,
        Some((&content_type, &body)),
        extra_headers,
        timeout,
    )
    .await
}

/// A multipart boundary tag unique per process invocation.
pub(crate) fn multipart_boundary() -> String {
    format!(
        "raydriver{}-{}",
        std::process::id(),
        REQUEST_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
    )
}

static REQUEST_COUNTER: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);

async fn request(
    host: &str,
    port: u16,
    method: &str,
    path: &str,
    body: Option<(&str, &[u8])>,
    extra_headers: &[(&str, &str)],
    timeout: Duration,
) -> Result<HttpResponse, DriverError> {
    tokio::time::timeout(timeout, async {
        let stream = crate::driver::core::connector::dial(host, port)
            .await
            .map_err(|e| DriverError::Connection(e.to_string()))?;
        exchange(
            stream,
            &format!("{host}:{port}"),
            method,
            path,
            body,
            extra_headers,
        )
        .await
        .map_err(|e| DriverError::Connection(e.to_string()))
    })
    .await
    .map_err(|_| DriverError::Timeout("operation timed out".to_string()))?
}

/// Run a single HTTP exchange over an existing byte stream.
///
/// *host* is used verbatim as the `Host` header (append `:port` if
/// the endpoint is not on the default port).  The request is written
/// with `Connection: close` semantics and the response is read to
/// EOF, so the stream is consumed afterwards.
pub async fn request_on_stream<S: AsyncRead + AsyncWrite + Unpin>(
    stream: S,
    host: &str,
    method: &str,
    path: &str,
    body: Option<(&str, &[u8])>,
    extra_headers: &[(&str, &str)],
) -> Result<HttpResponse, DriverError> {
    exchange(stream, host, method, path, body, extra_headers)
        .await
        .map_err(|e| DriverError::Connection(e.to_string()))
}

async fn exchange<S: AsyncRead + AsyncWrite + Unpin>(
    mut stream: S,
    host_header: &str,
    method: &str,
    path: &str,
    body: Option<(&str, &[u8])>,
    extra_headers: &[(&str, &str)],
) -> Result<HttpResponse, std::io::Error> {
    let mut req = format!("{method} {path} HTTP/1.1\r\n");
    req.push_str(&format!("Host: {host_header}\r\n"));
    req.push_str("Accept: */*\r\n");
    req.push_str("Connection: close\r\n");
    for (name, value) in extra_headers {
        req.push_str(&format!("{name}: {value}\r\n"));
    }
    match body {
        Some((content_type, payload)) => {
            req.push_str(&format!("Content-Type: {content_type}\r\n"));
            req.push_str(&format!("Content-Length: {}\r\n", payload.len()));
        }
        None => {
            req.push_str("Content-Length: 0\r\n");
        }
    }
    req.push_str("\r\n");
    stream.write_all(req.as_bytes()).await?;
    if let Some((_, payload)) = body {
        stream.write_all(payload).await?;
    }
    stream.flush().await?;

    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await?;
    parse_response(&raw)
}

fn parse_response(raw: &[u8]) -> Result<HttpResponse, std::io::Error> {
    let split = find_header_end(raw).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "malformed HTTP response: no header terminator",
        )
    })?;
    let (header_bytes, body_bytes) = raw.split_at(split);
    let headers = String::from_utf8_lossy(header_bytes);
    let mut lines = headers.split("\r\n");
    let status_line = lines.next().unwrap_or_default();
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("malformed HTTP status line: {status_line:?}"),
            )
        })?;

    let mut content_length: Option<usize> = None;
    let mut chunked = false;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim().to_ascii_uppercase();
        let value = value.trim();
        match name.as_str() {
            "CONTENT-LENGTH" => {
                content_length = value.parse::<usize>().ok();
            }
            "TRANSFER-ENCODING" => {
                chunked = value.to_ascii_lowercase().contains("chunked");
            }
            _ => {}
        }
    }

    let body = if chunked {
        decode_chunked(body_bytes)?
    } else if let Some(length) = content_length {
        body_bytes[..length.min(body_bytes.len())].to_vec()
    } else {
        body_bytes.to_vec()
    };
    Ok(HttpResponse { status, body })
}

/// The number of bytes up to and including the `\r\n\r\n` header
/// terminator.
fn find_header_end(raw: &[u8]) -> Option<usize> {
    raw.windows(4).position(|w| w == b"\r\n\r\n").map(|i| i + 4)
}

fn decode_chunked(mut body: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    let mut out = Vec::new();
    while let Some(line_end) = find_crlf(body) {
        let size_line = String::from_utf8_lossy(&body[..line_end]);
        let size_str = size_line.split(';').next().unwrap_or("").trim();
        let size = usize::from_str_radix(size_str, 16).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("malformed chunk size: {size_str:?}"),
            )
        })?;
        body = &body[line_end + 2..];
        if size == 0 {
            break;
        }
        let take = size.min(body.len());
        out.extend_from_slice(&body[..take]);
        body = &body[(take + 2).min(body.len())..];
    }
    Ok(out)
}

fn find_crlf(raw: &[u8]) -> Option<usize> {
    raw.windows(2).position(|w| w == b"\r\n")
}
