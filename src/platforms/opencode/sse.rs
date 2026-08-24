//! Minimal HTTP client for the opencode server on localhost.
//!
//! Supports plain GETs and the server's SSE `/event` stream over a raw
//! `TcpStream`, so the platform needs no extra HTTP dependencies. Handles
//! both chunked and close-delimited response bodies.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{bail, Context, Result};

/// SSE reads time out after this long with no data (heartbeats arrive well
/// inside it); the caller reconnects on timeout.
const STREAM_READ_TIMEOUT: Duration = Duration::from_secs(120);

/// Incremental decoder for `Transfer-Encoding: chunked` bodies.
#[derive(Default)]
struct ChunkedDecoder {
    state: ChunkState,
    remaining: usize,
}

#[derive(Default, PartialEq)]
enum ChunkState {
    #[default]
    Size,
    SizeLf,
    Data,
    DataCr,
    DataLf,
    Done,
}

impl ChunkedDecoder {
    /// Feed raw bytes, appending decoded payload bytes to `out`.
    /// Returns true once the terminating zero-length chunk is seen.
    fn feed(&mut self, input: &[u8], out: &mut Vec<u8>) -> Result<bool> {
        let mut pos = 0;
        while pos < input.len() {
            match self.state {
                ChunkState::Size => {
                    let byte = input[pos];
                    pos += 1;
                    match byte {
                        b'\r' => self.state = ChunkState::SizeLf,
                        b'0'..=b'9' => {
                            self.remaining = self.remaining * 16 + (byte - b'0') as usize
                        }
                        b'a'..=b'f' => {
                            self.remaining = self.remaining * 16 + (byte - b'a' + 10) as usize
                        }
                        b'A'..=b'F' => {
                            self.remaining = self.remaining * 16 + (byte - b'A' + 10) as usize
                        }
                        // Chunk extensions (";...") are ignored until CR
                        b';' => {
                            while pos < input.len() && input[pos] != b'\r' {
                                pos += 1;
                            }
                        }
                        _ => bail!("invalid chunk size byte: {byte:#x}"),
                    }
                }
                ChunkState::SizeLf => {
                    if input[pos] != b'\n' {
                        bail!("missing LF after chunk size");
                    }
                    pos += 1;
                    if self.remaining == 0 {
                        self.state = ChunkState::Done;
                        return Ok(true);
                    }
                    self.state = ChunkState::Data;
                }
                ChunkState::Data => {
                    let take = self.remaining.min(input.len() - pos);
                    out.extend_from_slice(&input[pos..pos + take]);
                    pos += take;
                    self.remaining -= take;
                    if self.remaining == 0 {
                        self.state = ChunkState::DataCr;
                    }
                }
                ChunkState::DataCr => {
                    if input[pos] != b'\r' {
                        bail!("missing CR after chunk data");
                    }
                    pos += 1;
                    self.state = ChunkState::DataLf;
                }
                ChunkState::DataLf => {
                    if input[pos] != b'\n' {
                        bail!("missing LF after chunk data");
                    }
                    pos += 1;
                    self.state = ChunkState::Size;
                }
                ChunkState::Done => return Ok(true),
            }
        }
        Ok(self.state == ChunkState::Done)
    }
}

struct Response {
    stream: TcpStream,
    chunked: bool,
    /// Body bytes that arrived in the same reads as the headers
    initial_body: Vec<u8>,
}

fn request(port: u16, path: &str, read_timeout: Duration) -> Result<Response> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .with_context(|| format!("connect to opencode server on port {port}"))?;
    stream.set_read_timeout(Some(read_timeout))?;
    stream.set_nodelay(true).ok();

    let req = format!(
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAccept: */*\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(req.as_bytes())?;

    // Read until the end of the response headers
    let mut buf = Vec::with_capacity(4096);
    let mut chunk = [0u8; 4096];
    let header_end = loop {
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            bail!("connection closed before response headers");
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(pos) = find_header_end(&buf) {
            break pos;
        }
        if buf.len() > 64 * 1024 {
            bail!("response headers too large");
        }
    };

    let headers = String::from_utf8_lossy(&buf[..header_end]).to_lowercase();
    let status_ok = headers
        .lines()
        .next()
        .is_some_and(|line| line.contains(" 200"));
    if !status_ok {
        bail!(
            "opencode server returned non-200: {}",
            headers.lines().next().unwrap_or("")
        );
    }
    let chunked = headers.contains("transfer-encoding: chunked");

    Ok(Response {
        stream,
        chunked,
        initial_body: buf[header_end + 4..].to_vec(),
    })
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

/// GET a JSON endpoint and parse the body.
pub fn get_json(port: u16, path: &str) -> Result<serde_json::Value> {
    let mut response = request(port, path, Duration::from_secs(10))?;
    let mut raw = response.initial_body;
    let mut chunk = [0u8; 16384];
    loop {
        match response.stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => raw.extend_from_slice(&chunk[..n]),
            Err(e) => return Err(e).context("read response body"),
        }
    }

    let body = if response.chunked {
        let mut decoder = ChunkedDecoder::default();
        let mut out = Vec::with_capacity(raw.len());
        decoder.feed(&raw, &mut out)?;
        out
    } else {
        raw
    };
    serde_json::from_slice(&body).context("parse JSON response")
}

/// Incremental SSE decoder: raw response bytes in, `data:` payloads out.
struct EventDecoder {
    chunked: bool,
    chunks: ChunkedDecoder,
    decoded: Vec<u8>,
    pending_line: String,
    event_data: String,
}

impl EventDecoder {
    fn new(chunked: bool) -> Self {
        Self {
            chunked,
            chunks: ChunkedDecoder::default(),
            decoded: Vec::new(),
            pending_line: String::new(),
            event_data: String::new(),
        }
    }

    /// Feed raw bytes, invoking `on_event` for each complete event.
    fn feed(&mut self, raw: &[u8], on_event: &mut dyn FnMut(&str)) -> Result<()> {
        self.decoded.clear();
        if self.chunked {
            self.chunks.feed(raw, &mut self.decoded)?;
        } else {
            self.decoded.extend_from_slice(raw);
        }
        self.pending_line
            .push_str(&String::from_utf8_lossy(&self.decoded));

        while let Some(newline) = self.pending_line.find('\n') {
            let line: String = self.pending_line.drain(..=newline).collect();
            let line = line.trim_end_matches(['\n', '\r']);
            if let Some(data) = line.strip_prefix("data:") {
                if !self.event_data.is_empty() {
                    self.event_data.push('\n');
                }
                self.event_data
                    .push_str(data.strip_prefix(' ').unwrap_or(data));
            } else if line.is_empty() && !self.event_data.is_empty() {
                on_event(&self.event_data);
                self.event_data.clear();
            }
        }
        Ok(())
    }
}

/// Follow an SSE stream, invoking `on_event` with each `data:` payload.
/// Returns when the connection drops, times out, or `shutdown` is set.
pub fn stream_events(
    port: u16,
    path: &str,
    shutdown: &AtomicBool,
    on_event: &mut dyn FnMut(&str),
) -> Result<()> {
    let mut response = request(port, path, STREAM_READ_TIMEOUT)?;
    let mut decoder = EventDecoder::new(response.chunked);

    let initial = std::mem::take(&mut response.initial_body);
    decoder.feed(&initial, on_event)?;

    let mut chunk = [0u8; 16384];
    while !shutdown.load(Ordering::Relaxed) {
        let n = match response.stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => {
                if shutdown.load(Ordering::Relaxed) {
                    break;
                }
                return Err(e).context("read event stream");
            }
        };
        decoder.feed(&chunk[..n], on_event)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_chunked_body_across_feeds() {
        let mut decoder = ChunkedDecoder::default();
        let mut out = Vec::new();
        // "5\r\nhello\r\n6\r\n world\r\n0\r\n\r\n" split awkwardly
        assert!(!decoder.feed(b"5\r\nhel", &mut out).unwrap());
        assert!(!decoder.feed(b"lo\r\n6\r", &mut out).unwrap());
        assert!(!decoder.feed(b"\n world\r\n", &mut out).unwrap());
        assert!(decoder.feed(b"0\r\n\r\n", &mut out).unwrap());
        assert_eq!(out, b"hello world");
    }

    #[test]
    fn decodes_chunk_extensions_and_hex_sizes() {
        let mut decoder = ChunkedDecoder::default();
        let mut out = Vec::new();
        let body = b"A;ext=1\r\n0123456789\r\n0\r\n\r\n";
        assert!(decoder.feed(body, &mut out).unwrap());
        assert_eq!(out, b"0123456789");
    }

    #[test]
    fn rejects_garbage_chunk_size() {
        let mut decoder = ChunkedDecoder::default();
        let mut out = Vec::new();
        assert!(decoder.feed(b"zz\r\n", &mut out).is_err());
    }
}
