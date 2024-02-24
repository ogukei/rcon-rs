use std::ffi::CString;

use crate::{client::RconClient, serialize::{Decode, Decoder, IoReader}, Packet, PacketType};

use anyhow::{bail, Result};
use log::trace;
use tokio::net::tcp::OwnedReadHalf;

#[derive(Debug)]
pub struct PacketDecodableAnySize {
    id: i32,
    r#type: PacketType,
    body: CString,
}

impl Decode for PacketDecodableAnySize {
    async fn decode(decoder: &mut impl Decoder) -> Result<Self> {
        trace!("decoding packet as PacketDecodableAnySize");
        let size = i32::decode(decoder).await?;
        trace!("ignoring packet size: {}", size);
        let id = i32::decode(decoder).await?;
        let r#type = i32::decode(decoder).await?;
        let r#type: PacketType = r#type.try_into()?;
        // read until null-terminated
        let mut body = vec![];
        let body = loop {
            let byte = u8::decode(decoder).await?;
            body.push(byte);
            if byte == 0 {
                break body
            }
            if body.len() > 4096 {
                bail!("broken packet: body too long")
            }
        };
        let body = CString::from_vec_with_nul(body)?;
        // empty string
        let null = u8::decode(decoder).await?;
        if null != 0 {
            bail!("broken packet: expected empty string")
        }
        let packet = PacketDecodableAnySize {
            id,
            r#type: r#type,
            body,
        };
        Ok(packet)
    }
}

impl RconClient {
    pub async fn read_packet_ignoring_size(&self) -> Result<Packet> {
        let mut stream_guard = self.read_stream.lock().await;
        let stream: &mut OwnedReadHalf = &mut *stream_guard;
        stream.readable().await?;
        let mut reader = IoReader::new(stream);
        let packet = PacketDecodableAnySize::decode(&mut reader).await?;
        let packet = Packet::with_raw_body(packet.id, packet.r#type, packet.body);
        Ok(packet)
    }
}
