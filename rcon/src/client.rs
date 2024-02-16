
use tokio::{io::{ReadHalf, WriteHalf}, net::TcpStream, sync::Mutex};
use anyhow::Result;

use crate::{packet::Packet, serialize::{Decode, Encode, IoReader, IoWriter}};

pub struct RconClient {
    read_stream: Mutex<ReadHalf<TcpStream>>,
    write_stream: Mutex<WriteHalf<TcpStream>>,
}

impl RconClient {
    pub async fn connect(endpoint: &str) -> Result<Self> {
        let stream = TcpStream::connect(endpoint).await?;
        let (read_stream, write_stream) = tokio::io::split(stream);
        let read_stream = Mutex::new(read_stream);
        let write_stream = Mutex::new(write_stream);
        let this = Self {
            read_stream,
            write_stream,
        };
        Ok(this)
    }

    pub async fn read_packet(&self) -> Result<Packet> {
        let mut stream_guard = self.read_stream.lock().await;
        let stream: &mut ReadHalf<TcpStream> = &mut *stream_guard;
        let mut reader = IoReader::new(stream);
        let packet = Packet::decode(&mut reader).await?;
        Ok(packet)
    }

    pub async fn write_packet(&self, packet: Packet) -> Result<()> {
        let mut stream_guard = self.write_stream.lock().await;
        let stream: &mut WriteHalf<TcpStream> = &mut *stream_guard;
        let mut writer = IoWriter::new(stream);
        packet.encode(&mut writer).await?;
        Ok(())
    }
}
