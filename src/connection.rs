use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;

pub enum Connection {
    Plain(BufReader<TcpStream>),
    Tls(BufReader<Box<TlsStream<TcpStream>>>),
}

impl Connection {
    pub async fn read_line(&mut self, buf: &mut String) -> tokio::io::Result<usize> {
        match self {
            Connection::Plain(stream) => stream.read_line(buf).await,
            Connection::Tls(stream) => stream.read_line(buf).await,
        }
    }

    pub async fn write_all(&mut self, buf: &[u8]) -> tokio::io::Result<()> {
        match self {
            Connection::Plain(stream) => stream.get_mut().write_all(buf).await,
            Connection::Tls(stream) => stream.get_mut().write_all(buf).await,
        }
    }

    pub async fn flush(&mut self) -> tokio::io::Result<()> {
        match self {
            Connection::Plain(stream) => stream.get_mut().flush().await,
            Connection::Tls(stream) => stream.get_mut().flush().await,
        }
    }

    pub fn into_plain_stream(self) -> Option<TcpStream> {
        match self {
            Connection::Plain(stream) => Some(stream.into_inner()),
            Connection::Tls(_) => None,
        }
    }
}
