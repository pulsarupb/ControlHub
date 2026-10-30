use crate::NtripConfig;
use base64::{Engine, engine::general_purpose::STANDARD};
use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

pub struct NtripClient {
    stream: TcpStream,
    last_data: Instant,
}

impl NtripClient {
    pub fn connect(config: &NtripConfig) -> io::Result<Self> {
        config.validate().map_err(io::Error::other)?;
        let address = (config.host.as_str(), config.port)
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| io::Error::other("no caster address"))?;
        let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(3))?;
        stream.set_read_timeout(Some(Duration::from_millis(100)))?;
        stream.set_write_timeout(Some(Duration::from_secs(3)))?;
        let auth = STANDARD.encode(format!("{}:{}", config.username, config.password));
        write!(
            stream,
            "GET /{} HTTP/1.0\r\nHost: {}\r\nUser-Agent: NTRIP ControlHub/1\r\nAuthorization: Basic {}\r\n\r\n",
            config.mount_point.trim_start_matches('/'),
            config.host,
            auth
        )?;
        handshake(&mut stream)?;
        stream.set_read_timeout(Some(Duration::from_millis(10)))?;
        Ok(Self {
            stream,
            last_data: Instant::now(),
        })
    }

    /// Empty means idle, not disconnected. EOF is an error so callers reconnect.
    pub fn read(&mut self) -> io::Result<Vec<u8>> {
        if self.last_data.elapsed() > Duration::from_secs(15) {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "caster correction stream stalled",
            ));
        }
        let mut bytes = vec![0; 4096];
        match self.stream.read(&mut bytes) {
            Ok(0) => Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "caster disconnected",
            )),
            Ok(n) => {
                self.last_data = Instant::now();
                bytes.truncate(n);
                Ok(bytes)
            }
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                Ok(vec![])
            }
            Err(e) => Err(e),
        }
    }
}

fn handshake(stream: &mut impl Read) -> io::Result<()> {
    let mut header = Vec::new();
    let mut byte = [0];
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "NTRIP handshake timed out",
            ));
        }
        match stream.read_exact(&mut byte) {
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock
                        | io::ErrorKind::TimedOut
                        | io::ErrorKind::Interrupted
                ) =>
            {
                continue;
            }
            result => result?,
        }
        header.push(byte[0]);
        if header.len() > 8192 {
            return Err(io::Error::other("oversized NTRIP header"));
        }
        if !header.ends_with(b"\r\n") {
            continue;
        }
        let first_end = header.windows(2).position(|w| w == b"\r\n").unwrap();
        let status = String::from_utf8_lossy(&header[..first_end]).into_owned();
        let mut parts = status.split_whitespace();
        let protocol = parts.next().unwrap_or("");
        if !matches!(protocol, "ICY" | "HTTP/1.0" | "HTTP/1.1") || parts.next() != Some("200") {
            return Err(io::Error::other(format!("NTRIP rejected stream: {status}")));
        }
        // NTRIP v1 ICY has only a status line. HTTP needs the full header.
        if protocol == "ICY" || header.ends_with(b"\r\n\r\n") {
            let text = String::from_utf8_lossy(&header).to_ascii_lowercase();
            if text.contains("sourcetable") || text.contains("transfer-encoding:") {
                return Err(io::Error::other(
                    "caster returned a source table or unsupported transfer encoding",
                ));
            }
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_icy_http_and_leaves_binary_payload_unread() {
        for header in ["ICY 200 OK\r\n", "HTTP/1.0 200 OK\r\nServer: test\r\n\r\n"] {
            let mut cursor = io::Cursor::new([header.as_bytes(), &[0xd3, 0, 1]].concat());
            handshake(&mut cursor).unwrap();
            let mut payload = vec![];
            cursor.read_to_end(&mut payload).unwrap();
            assert_eq!(payload, [0xd3, 0, 1]);
        }
    }
    #[test]
    fn rejects_source_table_auth_error_and_false_200() {
        for response in [
            "SOURCETABLE 200 OK\r\n",
            "HTTP/1.0 401 Unauthorized\r\n",
            "HTTP/1.0 1200 Bad\r\n",
            "HTTP/1.1 200 OK\r\nContent-Type: gnss/sourcetable\r\n\r\n",
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n",
        ] {
            assert!(handshake(&mut io::Cursor::new(response)).is_err());
        }
        assert!(handshake(&mut io::Cursor::new(vec![b'x'; 8193])).is_err());
    }

    #[test]
    fn local_caster_auth_binary_payload_and_disconnect() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = Vec::new();
            let mut byte = [0];
            while !request.ends_with(b"\r\n\r\n") {
                stream.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
            }
            let request = String::from_utf8(request).unwrap();
            assert!(request.starts_with("GET /test HTTP/1.0\r\n"));
            assert!(request.contains("Authorization: Basic dXNlcjpwYXNz\r\n"));
            stream.write_all(b"ICY 200 OK\r\n\xd3\0\x01").unwrap();
        });
        let config = NtripConfig {
            host: "127.0.0.1".into(),
            port,
            mount_point: "test".into(),
            username: "user".into(),
            password: "pass".into(),
            ..Default::default()
        };
        let mut client = NtripClient::connect(&config).unwrap();
        server.join().unwrap();
        let mut payload = vec![];
        loop {
            match client.read() {
                Ok(bytes) => payload.extend(bytes),
                Err(e) => {
                    assert_eq!(e.kind(), io::ErrorKind::UnexpectedEof);
                    break;
                }
            }
        }
        assert_eq!(payload, [0xd3, 0, 1]);
    }
}
