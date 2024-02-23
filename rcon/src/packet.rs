
use std::ffi::CString;
use anyhow::{bail, Result};
use crate::serialize::{Encode, Decode, Encoder, Decoder};
use log::trace;

// https://developer.valvesoftware.com/wiki/Source_RCON_Protocol
#[derive(Debug)]
pub struct Packet {
    id: i32,
    r#type: PacketType,
    body: CString,
}

impl Packet {
    pub fn new(id: i32, r#type: PacketType, body: String) -> Result<Packet>  {
        let value = Packet {
            id,
            r#type,
            body: CString::new(body.as_str())?,
        };
        Ok(value)
    }

    pub fn with_raw_body(id: i32, r#type: PacketType, body: CString) -> Packet  {
        Packet {
            id,
            r#type,
            body,
        }
    }

    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn r#type(&self) -> PacketType {
        self.r#type
    }

    pub fn body(&self) -> Result<String> {
        let body = self.body.to_str()?;
        Ok(body.to_owned())
    }

    pub fn raw_body(&self) -> &CString {
        &self.body
    }
}

impl Encode for Packet {
    async fn encode(&self, encoder: &mut impl Encoder) -> Result<()> {
        let size: usize = 8 + self.body.as_bytes_with_nul().len() + 1;
        let size = size as i32;
        size.encode(encoder).await?;
        self.id.encode(encoder).await?;
        let r#type: i32 = self.r#type as i32;
        r#type.encode(encoder).await?;
        encoder.write(self.body.as_bytes_with_nul()).await?;
        0u8.encode(encoder).await?;
        Ok(())
    }
}

impl Decode for Packet {
    async fn decode(decoder: &mut impl Decoder) -> Result<Self> {
        trace!("decoding packet");
        let size = i32::decode(decoder).await?;
        let id = i32::decode(decoder).await?;
        let r#type = i32::decode(decoder).await?;
        let r#type: PacketType = r#type.try_into()?;
        // without id, type and an empty string
        let body_size = size - (8 + 1);
        if body_size <= 0 || body_size >= 4096 {
            bail!("broken packet: invalid size")
        }
        trace!("reading body {} bytes", body_size);
        let body_size = body_size as usize;
        // body string
        let mut body: Vec<u8> = vec![0u8; body_size];
        decoder.read(&mut body).await?;
        let body = CString::from_vec_with_nul(body)?;
        // empty string
        let null = u8::decode(decoder).await?;
        if null != 0 {
            bail!("broken packet: expected empty string")
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
pub enum PacketType {
    Auth = 3,
    ExecCommandOrAuthResponse = 2,
    ResponseValue = 0,
    AuthFailed = -1,
}

impl PacketType {
    pub const AUTH: Self = Self::Auth;
    pub const EXEC_COMMAND: Self = Self::ExecCommandOrAuthResponse;
    pub const AUTH_RESPONSE: Self = Self::ExecCommandOrAuthResponse;
    pub const RESPONSE_VALUE: Self = Self::ResponseValue;
    pub const AUTH_FAILED: Self = Self::AuthFailed;
}

impl TryFrom<i32> for PacketType {
    type Error = anyhow::Error;

    fn try_from(value: i32) -> Result<Self> {
        let value = match value {
            3 => Self::Auth,
            2 => Self::ExecCommandOrAuthResponse,
            0 => Self::ResponseValue,
            -1 => Self::AuthFailed,
            _ => bail!("invalid packet type"),
        };
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use crate::serialize::decode_from_slice;
    use crate::serialize::encode_to_vec;
    use super::{Packet, PacketType};

    #[tokio::test]
    async fn test_decode() {
        let data: Vec<u8> = vec![0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x70, 0x61, 0x73, 0x73, 0x77, 0x72, 0x64, 0x00, 0x00];
        let packet: Packet = decode_from_slice(&data).await.unwrap();
        assert_eq!(packet.id, 0);
        assert_eq!(packet.r#type, PacketType::Auth);
        assert_eq!(packet.body.to_str().unwrap(), "passwrd");
    }

    #[tokio::test]
    async fn test_encode() {
        let packet = Packet::new(0, PacketType::Auth, "passwrd".into()).unwrap();
        let data = encode_to_vec(&packet).await.unwrap();
        assert_eq!(data, vec![0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x70, 0x61, 0x73, 0x73, 0x77, 0x72, 0x64, 0x00, 0x00]);
    }
}
