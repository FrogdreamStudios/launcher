//! Simple HTTP client with HTTPS support.

use std::collections::HashMap;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream as TokioTcpStream;
use tokio_native_tls::{TlsConnector, TlsStream};

use crate::utils::Result;
use crate::{log_debug, simple_error};

/// Simple HTTP client with HTTPS support.
#[derive(Clone)]
pub struct Client {
    timeout: Duration,
    connect_timeout: Duration,
    user_agent: String,
}

/// HTTP response.
pub struct Response {
    status: u16,
    body: Vec<u8>,
}

/// HTTP status codes we care about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum StatusCode {
    Ok = 200,
    TooManyRequests = 429,
    Other(u16),
}

impl StatusCode {
    pub const fn from_u16(code: u16) -> Self {
        match code {
            200 => Self::Ok,
            429 => Self::TooManyRequests,
            other => Self::Other(other),
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(self, Self::Ok) || matches!(self, Self::Other(code) if (200..300).contains(code))
    }

    pub const fn as_u16(&self) -> u16 {
        match self {
            Self::Ok => 200,
            Self::TooManyRequests => 429,
            Self::Other(code) => *code,
        }
    }
}

impl std::fmt::Display for StatusCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_u16())
    }
}

impl Response {
    /// Get the status code.
    pub fn status(&self) -> StatusCode {
        StatusCode::from_u16(self.status)
    }

    /// Deserialize the response body as JSON.
    pub fn json<T>(&self) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let text = String::from_utf8(self.body.clone())
            .map_err(|_e| simple_error!("Invalid UTF-8 in response: {e}"))?;

        serde_json::from_str(&text).map_err(|_e| simple_error!("JSON parse error: {e}"))
    }

    /// Get the next chunk of data (for streaming).
    pub fn chunk(&mut self) -> Result<Option<Vec<u8>>> {
        // Return all remaining data as one chunk for simplicity
        // In a real streaming implementation, this would read chunks from the network
        if self.body.is_empty() {
            Ok(None)
        } else {
            let chunk = std::mem::take(&mut self.body);
            Ok(Some(chunk))
        }
    }
}

impl Client {
    /// Create a new HTTP client.
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
            user_agent: "SimpleHTTP/1.0".to_string(),
        }
    }

    /// Create a client builder for configuration.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// Perform a GET request.
    pub async fn get(&self, url: &str) -> Result<Response> {
        self.request("GET", url, None).await
    }

    /// Perform an HTTP request.
    async fn request(&self, method: &str, url: &str, body: Option<&[u8]>) -> Result<Response> {
        let parsed_url = parse_url(url)?;

        log_debug!("Making {method} request to {url}");

        // Resolve the address
        let addr = format!("{}:{}", parsed_url.host, parsed_url.port);
        let socket_addr = tokio::net::lookup_host(&addr)
            .await
            .map_err(|e| simple_error!("DNS lookup failed: {}", e))?
            .next()
            .ok_or_else(|| simple_error!("Could not resolve host: {}", parsed_url.host))?;

        // Connect with timeout
        let tcp_stream =
            tokio::time::timeout(self.connect_timeout, TokioTcpStream::connect(socket_addr))
                .await
                .map_err(|_| simple_error!("Connection timeout"))?
                .map_err(|e| simple_error!("Connection failed: {}", e))?;

        // Determine if we need TLS
        let response_data = if parsed_url.is_https {
            // HTTPS connection
            let connector = TlsConnector::from(
                native_tls::TlsConnector::new()
                    .map_err(|e| simple_error!("TLS connector creation failed: {}", e))?,
            );

            let tls_stream = connector
                .connect(&parsed_url.host, tcp_stream)
                .await
                .map_err(|e| simple_error!("TLS handshake failed: {}", e))?;

            self.send_request_and_read_response(
                method,
                &parsed_url,
                body,
                StreamWrapper::Tls(tls_stream),
            )
            .await?
        } else {
            // HTTP connection
            self.send_request_and_read_response(
                method,
                &parsed_url,
                body,
                StreamWrapper::Plain(tcp_stream),
            )
            .await?
        };

        // Parse HTTP response
        parse_response(response_data)
    }

    async fn send_request_and_read_response(
        &self,
        method: &str,
        parsed_url: &ParsedUrl,
        body: Option<&[u8]>,
        mut stream: StreamWrapper,
    ) -> Result<Vec<u8>> {
        // Build HTTP request
        let mut request = format!(
            "{} {} HTTP/1.1\r\n\
             Host: {}\r\n\
             User-Agent: {}\r\n\
             Connection: close\r\n",
            method, parsed_url.path, parsed_url.host, self.user_agent
        );

        if let Some(body_data) = body {
            request.push_str(&format!("Content-Length: {}\r\n", body_data.len()));
        }

        request.push_str("\r\n");

        // Send request with timeout
        tokio::time::timeout(self.timeout, async {
            // Send headers
            stream.write_all(request.as_bytes()).await?;

            // Send body if present
            if let Some(body_data) = body {
                stream.write_all(body_data).await?;
            }

            // Read response
            let mut response_data = Vec::new();
            stream.read_to_end(&mut response_data).await?;

            Ok::<Vec<u8>, std::io::Error>(response_data)
        })
        .await
        .map_err(|_| simple_error!("Request timeout"))?
        .map_err(|e| simple_error!("Request failed: {}", e))
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

/// Client builder for configuration.
pub struct ClientBuilder {
    timeout: Duration,
    connect_timeout: Duration,
    user_agent: String,
}

impl ClientBuilder {
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
            user_agent: "SimpleHTTP/1.0".to_string(),
        }
    }

    pub const fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub const fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    pub fn user_agent(mut self, user_agent: &str) -> Self {
        self.user_agent = user_agent.to_string();
        self
    }

    pub fn build(self) -> Result<Client> {
        Ok(Client {
            timeout: self.timeout,
            connect_timeout: self.connect_timeout,
            user_agent: self.user_agent,
        })
    }
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Wrapper for both plain TCP and TLS streams.
enum StreamWrapper {
    Plain(TokioTcpStream),
    Tls(TlsStream<TokioTcpStream>),
}

impl tokio::io::AsyncRead for StreamWrapper {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match &mut *self {
            Self::Plain(stream) => std::pin::Pin::new(stream).poll_read(cx, buf),
            Self::Tls(stream) => std::pin::Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl tokio::io::AsyncWrite for StreamWrapper {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::result::Result<usize, std::io::Error>> {
        match &mut *self {
            Self::Plain(stream) => std::pin::Pin::new(stream).poll_write(cx, buf),
            Self::Tls(stream) => std::pin::Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::result::Result<(), std::io::Error>> {
        match &mut *self {
            Self::Plain(stream) => std::pin::Pin::new(stream).poll_flush(cx),
            Self::Tls(stream) => std::pin::Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::result::Result<(), std::io::Error>> {
        match &mut *self {
            Self::Plain(stream) => std::pin::Pin::new(stream).poll_shutdown(cx),
            Self::Tls(stream) => std::pin::Pin::new(stream).poll_shutdown(cx),
        }
    }
}

/// Simple URL parsing with HTTPS support.
struct ParsedUrl {
    host: String,
    port: u16,
    path: String,
    is_https: bool,
}

fn parse_url(url: &str) -> Result<ParsedUrl> {
    let is_https = url.starts_with("https://");
    let is_http = url.starts_with("http://");

    if !is_https && !is_http {
        return Err(simple_error!("URL must start with http:// or https://"));
    }

    let url = if is_https {
        url.strip_prefix("https://").unwrap()
    } else {
        url.strip_prefix("http://").unwrap()
    };

    let (host_port, path) = if let Some(slash_pos) = url.find('/') {
        (&url[..slash_pos], &url[slash_pos..])
    } else {
        (url, "/")
    };

    let (host, port) = if let Some(colon_pos) = host_port.rfind(':') {
        let host = &host_port[..colon_pos];
        let port_str = &host_port[colon_pos + 1..];
        let port = port_str
            .parse::<u16>()
            .map_err(|_| simple_error!("Invalid port: {port_str}"))?;
        (host.to_string(), port)
    } else {
        let default_port = if is_https { 443 } else { 80 };
        (host_port.to_string(), default_port)
    };

    Ok(ParsedUrl {
        host,
        port,
        path: path.to_string(),
        is_https,
    })
}

fn parse_response(data: Vec<u8>) -> Result<Response> {
    // Find the end of headers (double CRLF or double LF)
    let header_end = find_header_end(&data)?;

    // Split headers and body
    let headers_data = &data[..header_end];
    let body = if header_end + 4 <= data.len() && &data[header_end..header_end + 4] == b"\r\n\r\n" {
        data[header_end + 4..].to_vec()
    } else if header_end + 2 <= data.len() && &data[header_end..header_end + 2] == b"\n\n" {
        data[header_end + 2..].to_vec()
    } else {
        Vec::new()
    };

    // Parse headers as UTF-8 (headers should always be UTF-8)
    let headers_str = String::from_utf8(headers_data.to_vec())
        .map_err(|_| simple_error!("Invalid UTF-8 in response headers"))?;

    let mut lines = headers_str.lines();

    // Parse status line
    let status_line = lines
        .next()
        .ok_or_else(|| simple_error!("Empty response"))?;

    let status_parts: Vec<&str> = status_line.split_whitespace().collect();
    if status_parts.len() < 2 {
        return Err(simple_error!("Invalid status line: {status_line}"));
    }

    let status: u16 = status_parts[1]
        .parse()
        .map_err(|_| simple_error!("Invalid status code: {}", status_parts[1]))?;

    // Parse headers
    let mut headers = HashMap::new();

    for line in lines {
        if line.is_empty() {
            // End of headers
            break;
        }

        if let Some(colon_pos) = line.find(':') {
            let key = line[..colon_pos].trim().to_string();
            let value = line[colon_pos + 1..].trim().to_string();
            headers.insert(key.to_lowercase(), value);
        }
    }

    Ok(Response { status, body })
}

/// Find the end of HTTP headers in the response data.
fn find_header_end(data: &[u8]) -> Result<usize> {
    // Look for \r\n\r\n
    for i in 0..data.len().saturating_sub(3) {
        if &data[i..i + 4] == b"\r\n\r\n" {
            return Ok(i);
        }
    }

    // Fallback: look for \n\n
    for i in 0..data.len().saturating_sub(1) {
        if &data[i..i + 2] == b"\n\n" {
            return Ok(i);
        }
    }

    // If no header separator found, treat entire data as headers
    Ok(data.len())
}

/// Simple error type for HTTP operations.
#[derive(Debug)]
pub struct Error {
    message: String,
}

impl Error {
    pub const fn new(message: String) -> Self {
        Self { message }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::new(format!("IO error: {err}"))
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::new(format!("JSON error: {err}"))
    }
}
