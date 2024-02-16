

use std::sync::{Arc, Mutex};
use bincode::{config, de::{read::Reader, DecoderImpl}, enc::{write::Writer, EncoderImpl}, error::{DecodeError, EncodeError}, Decode, Encode};

use tokio::net::TcpStream;
use tokio_util::io::SyncIoBridge;
use tokio::task;

use anyhow::Result;

use crate::packet::Packet;

pub struct RconClient {
    stream: Arc<Mutex<TcpStream>>,
}

impl RconClient {
    pub async fn connect(endpoint: &str) -> Result<Self> {
        let stream = TcpStream::connect(endpoint).await?;
        let stream = Arc::new(Mutex::new(stream));
        let this = Self {
            stream,
        };
        Ok(this)
    }

    pub fn stream(&self) -> &Arc<Mutex<TcpStream>> {
        &self.stream
    }

    pub async fn read_packet(&self) -> Result<Packet> {
        self.read_decodable().await
    }

    async fn read_decodable<T>(&self) -> Result<T>
        where T: Decode + Send + 'static
    {
        let stream = Arc::clone(self.stream());
        let handle = task::spawn_blocking(move || {
            let mut stream_guard = stream.lock().unwrap();
            let stream: &mut TcpStream = &mut *stream_guard;
            let mut bridge = SyncIoBridge::new(stream);
            let reader = IoExactReader::new(&mut bridge);
            let mut decoder = DecoderImpl::new(reader, config::legacy());
            let value = T::decode(&mut decoder)?;
            Ok(value)
        });
        handle.await?
    }

    pub async fn write_packet(&self, packet: Packet) -> Result<()> {
        self.write_encodable(packet).await
    }

    async fn write_encodable<T>(&self, value: T) -> Result<()>
        where T: Encode + Send + 'static
    {
        let stream = Arc::clone(self.stream());
        let handle = task::spawn_blocking(move || {
            let mut stream_guard = stream.lock().unwrap();
            let stream: &mut TcpStream = &mut *stream_guard;
            let mut bridge = SyncIoBridge::new(stream);
            let writer = IoExactWriter::new(&mut bridge);
            let mut encoder = EncoderImpl::new(writer, config::legacy());
            value.encode(&mut encoder)?;
            Ok(())
        });
        handle.await?
    }
}

// std::io::Read wrapper exactly reading bytes
struct IoExactReader<T> {
    // expects &mut TcpStream
    inner: T,
}

impl<T> IoExactReader<T> {
    fn new(inner: T) -> Self {
        Self {
            inner,
        }
    }
}

impl<T: std::io::Read> Reader for IoExactReader<T> {
    fn read(&mut self, bytes: &mut [u8]) -> Result<(), DecodeError> {
        self.inner.read_exact(bytes)
            .map_err(|inner| DecodeError::Io {
                inner,
                additional: bytes.len(),
            })
    }
}

struct IoExactWriter<T> {
    // expects &mut TcpStream
    inner: T,
}

impl<T> IoExactWriter<T> {
    fn new(inner: T) -> Self {
        Self {
            inner,
        }
    }
}

impl<T: std::io::Write> Writer for IoExactWriter<T> {
    fn write(&mut self, bytes: &[u8]) -> Result<(), EncodeError> {
        self.inner.write_all(bytes)
            .map_err(|inner| EncodeError::Io {
                inner,
                index: 0,
            })
    }
}
