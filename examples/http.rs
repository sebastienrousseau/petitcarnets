//! A simple program that makes a GET request and prints the response
//! status: Bubble Tea's `http` example. The request is plain HTTP/1.1
//! over `std::net` (the standard library has no TLS, so `https://` URLs
//! are refused), on a task's thread, with a ten second timeout. Pass
//! another URL as the first argument.
//!
//! `cargo run --example http -- http://example.com/`

use petitcarnets as tea;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

const URL: &str = "http://charm.sh/";
const TIMEOUT: Duration = Duration::from_secs(10);

pub(crate) enum Msg {
    Status(u16),
    Err(String),
}

pub(crate) struct Model {
    pub(crate) url: String,
    pub(crate) status: u16,
    pub(crate) err: Option<String>,
}

impl Model {
    pub(crate) fn new(url: &str) -> Self {
        Self {
            url: url.to_owned(),
            status: 0,
            err: None,
        }
    }
}

impl tea::Model for Model {
    type Msg = Msg;

    fn init(&mut self) -> tea::Cmd<Msg> {
        let url = self.url.clone();
        tea::Cmd::task(move || match check_server(&url) {
            Ok(status) => Msg::Status(status),
            Err(err) => Msg::Err(err),
        })
    }

    fn update(&mut self, msg: tea::Msg<Msg>) -> tea::Cmd<Msg> {
        match msg {
            tea::Msg::KeyPress(k) if k.matches_any(&["q", "ctrl+c", "esc"]) => tea::quit(),
            tea::Msg::App(Msg::Status(status)) => {
                self.status = status;
                tea::quit()
            }
            tea::Msg::App(Msg::Err(err)) => {
                self.err = Some(err);
                tea::Cmd::none()
            }
            _ => tea::Cmd::none(),
        }
    }

    fn view(&self) -> tea::View {
        let result = if let Some(err) = &self.err {
            format!("something went wrong: {err}")
        } else if self.status != 0 {
            format!("{} {}", self.status, status_text(self.status))
        } else {
            String::new()
        };
        tea::View::new(format!("Checking {}...{result}\n", self.url))
    }
}

/// The host, port and path of a plain `http://` URL.
fn parse(url: &str) -> Result<(String, u16, String), String> {
    let rest = url
        .strip_prefix("http://")
        .ok_or("only http:// URLs: the standard library has no TLS")?;
    let (authority, path) = rest.find('/').map_or((rest, "/"), |i| rest.split_at(i));
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (
            host,
            port.parse().map_err(|_| format!("bad port {port:?}"))?,
        ),
        None => (authority, 80),
    };
    Ok((host.to_owned(), port, path.to_owned()))
}

/// Sends `GET url` and reads the status code from the reply.
fn check_server(url: &str) -> Result<u16, String> {
    let (host, port, path) = parse(url)?;
    let addr = (host.as_str(), port)
        .to_socket_addrs()
        .map_err(|e| e.to_string())?
        .next()
        .ok_or_else(|| format!("no address for {host}"))?;
    let mut stream = TcpStream::connect_timeout(&addr, TIMEOUT).map_err(|e| e.to_string())?;
    stream
        .set_read_timeout(Some(TIMEOUT))
        .map_err(|e| e.to_string())?;
    let request = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    stream
        .write_all(request.as_bytes())
        .map_err(|e| e.to_string())?;
    let mut line = String::new();
    let _ = BufReader::new(stream)
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;
    line.split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .ok_or_else(|| format!("not an HTTP reply: {:?}", line.trim_end()))
}

/// Go's `http.StatusText`, for the common codes.
fn status_text(code: u16) -> &'static str {
    match code {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        307 => "Temporary Redirect",
        308 => "Permanent Redirect",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "",
    }
}

// `pub(crate)`: `tests/examples.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let url = std::env::args().nth(1).unwrap_or_else(|| URL.to_owned());
    let _ = tea::Program::new(Model::new(&url)).run()?;
    Ok(())
}
