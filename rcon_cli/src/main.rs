
use std::{any, ffi::CString, io::BufReader, str::FromStr};
use bincode::{config, de::{read::Reader, Decoder}, enc::{write::Writer, Encoder}, error::{DecodeError, EncodeError}, Decode, Encode};

// https://developer.valvesoftware.com/wiki/Source_RCON_Protocol
#[derive(Debug)]
struct Packet {
    id: i32,
    r#type: PacketType,
    body: String,
}

impl Packet {
    fn new(id: i32, r#type: PacketType, body: String) -> Packet  {
        Packet {
            id,
            r#type,
            body,
        }
    }
}

impl Encode for Packet {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        let body = CString::new(self.body.as_str())
            .map_err(|_| EncodeError::Other("invalid body string"))?;
        let size: usize = 8 + body.as_bytes_with_nul().len() + 1;
        let size = size as i32;
        size.encode(encoder)?;
        self.id.encode(encoder)?;
        let r#type = self.r#type as i32;
        r#type.encode(encoder)?;
        body.as_bytes_with_nul().encode(encoder)?;
        0u8.encode(encoder)?;
        Ok(())
    }
}

impl Decode for Packet {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        let size = i32::decode(decoder)?;
        let id = i32::decode(decoder)?;
        let r#type = i32::decode(decoder)?;
        let r#type: PacketType = r#type.try_into()?;
        // without id, type and an empty string
        let body_size = size - (8 + 1);
        if body_size <= 0 {
            return Err(DecodeError::Other("broken packet: invalid size"))
        }
        let body_size = body_size as usize;
        // body string
        decoder.claim_bytes_read(body_size)?;
        let mut body: Vec<u8> = vec![0u8; body_size];
        decoder.reader().read(&mut body)?;
        let body = CString::from_vec_with_nul(body)
            .map_err(|_| DecodeError::CStringNulError {
                position: 0,
            })?;
        let body = body.to_str()
            .map_err(|_| DecodeError::Other("invalid body string"))?
            .to_owned();
        // empty string
        let null = u8::decode(decoder)?;
        if null != 0 {
            return Err(DecodeError::Other("broken packet: expected empty string"))
        }
        let packet = Packet {
            id,
            r#type: r#type,
            body,
        };
        Ok(packet)
    }
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq)]
enum PacketType {
    Auth = 3,
    ExecCommandOrAuthResponse = 2,
    ResponseValue = 0,
    AuthFailed = -1,
}

impl TryFrom<i32> for PacketType {
    type Error = DecodeError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        let value = match value {
            3 => Self::Auth,
            2 => Self::ExecCommandOrAuthResponse,
            0 => Self::ResponseValue,
            -1 => Self::AuthFailed,
            _ => return Err(DecodeError::Other("invalid packet type")),
        };
        Ok(value)
    }
}

use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;
use tokio_util::io::SyncIoBridge;
use std::io;

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut stream = TcpStream::connect("0.0.0.0:25575").await?;
    let packet = Packet::new(123, PacketType::Auth, "password".into());
    let packet = bincode::encode_to_vec(packet, config::legacy()).unwrap();
    stream.write_all(&packet).await?;
    let bridge = SyncIoBridge::new(stream);
    let mut reader = BufReader::with_capacity(4096, bridge);
    let (data, _): (Packet, usize) = bincode::decode_from_reader(&mut reader, config::legacy()).unwrap();
    println!("{:?}", data);
    Ok(())
}

#[cfg(test)]
mod tests {
    use bincode::config;
    use crate::PacketType;

    use super::Packet;

    #[test]
    fn it_works() {
        let data: Vec<u8> = vec![0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x70, 0x61, 0x73, 0x73, 0x77, 0x72, 0x64, 0x00, 0x00];
        let (data, _): (Packet, usize) = bincode::decode_from_slice(&data, config::legacy()).unwrap();
        println!("{:?}", data);
        assert_eq!(data.id, 0);
        assert_eq!(data.r#type, PacketType::Auth);
        assert_eq!(data.body, "passwrd");
    }
}
