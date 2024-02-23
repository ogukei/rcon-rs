
use tokio::{io::{AsyncWriteExt, BufWriter}, net::{tcp::{OwnedReadHalf, OwnedWriteHalf}, TcpStream}, sync::Mutex};
use anyhow::Result;

use crate::{packet::Packet, serialize::{Decode, Encode, IoReader, IoWriter}};

pub struct RconClient {
    read_stream: Mutex<OwnedReadHalf>,
    write_stream: Mutex<OwnedWriteHalf>,
}

impl RconClient {
    pub async fn connect(endpoint: &str) -> Result<Self> {
        let stream = TcpStream::connect(endpoint).await?;
        let (read_stream, write_stream) = stream.into_split();
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
        let stream: &mut OwnedReadHalf = &mut *stream_guard;
        stream.readable().await?;
        let mut reader = IoReader::new(stream);
        let packet = Packet::decode(&mut reader).await?;
        Ok(packet)
    }

    pub async fn write_packet(&self, packet: Packet) -> Result<()> {
        let mut stream_guard = self.write_stream.lock().await;
        let stream: &mut OwnedWriteHalf = &mut *stream_guard;
        stream.writable().await?;
        let buffer = BufWriter::new(stream);
        let mut writer = IoWriter::new(buffer);
        packet.encode(&mut writer).await?;
        let mut buffer = writer.into_inner();
        buffer.flush().await?;
        Ok(())
    }
}
