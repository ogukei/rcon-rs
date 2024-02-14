
use std::{any, env, ffi::CString, io::{BufReader, Read}, ptr::slice_from_raw_parts, str::FromStr, sync::{Arc, Mutex}};
use bincode::{config, de::{read::Reader, Decoder, DecoderImpl}, enc::{write::Writer, Encoder}, error::{DecodeError, EncodeError}, Decode, Encode};

fn from_u16(from: &[u16]) -> &[u8] {
    let len = from.len().checked_mul(2).unwrap();
    let ptr: *const u8 = from.as_ptr().cast();
    unsafe { std::slice::from_raw_parts(ptr, len) }
}

// https://developer.valvesoftware.com/wiki/Source_RCON_Protocol
#[derive(Debug)]
struct Packet {
    id: i32,
    r#type: PacketType,
    body: CString,
}

impl Packet {
    fn new_raw(id: i32, r#type: PacketType, body: CString) -> Packet  {
        Packet {
            id,
            r#type,
            body,
        }
    }

    fn new(id: i32, r#type: PacketType, body: String) -> Result<Packet, anyhow::Error>  {
        let value = Packet {
            id,
            r#type,
            body: CString::new(body.as_str())?,
        };
        Ok(value)
    }

    fn new_utf16(id: i32, r#type: PacketType, body: String) -> Result<Packet, anyhow::Error>  {
        let body: Vec<u16> = body.encode_utf16().chain(vec![0]).collect();
        let body = from_u16(&body);
        
        let value = Packet {
            id,
            r#type,
            body: CString::new(body)?,
        };
        Ok(value)
    }
}

impl Encode for Packet {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        let size: usize = 8 + self.body.as_bytes_with_nul().len() + 1;
        let size = size as i32;
        size.encode(encoder)?;
        self.id.encode(encoder)?;
        let r#type: i32 = self.r#type as i32;
        r#type.encode(encoder)?;
        encoder.writer().write(self.body.as_bytes_with_nul())?;
        0u8.encode(encoder)?;
        Ok(())
    }
}

impl Decode for Packet {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        let size = i32::decode(decoder)?;
        println!("size: {}", size);
        let id = i32::decode(decoder)?;
        println!("id: {}", id);
        let r#type = i32::decode(decoder)?;
        println!("type: {}", r#type);
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
        println!("body: {}", body.to_str().unwrap_or("-"));
        // empty string
        let null = u8::decode(decoder)?;
        if null != 0 {
            return Err(DecodeError::Other("broken packet: expected empty string"))
        }
        println!("null: {}", null);
        let packet = Packet {
            id,
            r#type: r#type,
            body,
        };
        println!("ok packet");
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
use tokio::task;
use std::io;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let endpoint = env::var("RCON_ENDPOINT")
        .expect("RCON_ENDPOINT is required");
    let stream = TcpStream::connect(endpoint).await?;
    let stream = Arc::new(Mutex::new(stream));
    println!("connected!");
    let password = env::var("RCON_PASSWORD")
        .expect("RCON_PASSWORD is required");
    let packet = Packet::new(123, PacketType::Auth, password.into())?;
    let packet = bincode::encode_to_vec(packet, config::legacy()).unwrap();
    {
        let mut guard = stream.lock().unwrap();
        let stream = &mut *guard;
        stream.write_all(&packet).await?;
        stream.readable().await?;
    }
    // read
    let data: Result<Packet, _> = decode_blocking(&stream).await;
    println!("reading done packet! {:?}", data);
    sleep(Duration::from_secs(1)).await;
    // write
    {
        let command = env::var("RCON_COMMAND")
            .unwrap_or("broadcast こんにちは".into());
        let packet = Packet::new(345, PacketType::ExecCommandOrAuthResponse, command)?;
        let packet = bincode::encode_to_vec(packet, config::legacy()).unwrap();
        let mut guard = stream.lock().unwrap();
        let stream = &mut *guard;
        stream.write_all(&packet).await?;
        stream.readable().await?;
        println!("writing packet done!");
        sleep(Duration::from_secs(1)).await;
    }
    Ok(())
}

async fn decode_blocking<T>(stream: &Arc<Mutex<TcpStream>>) -> Result<T, anyhow::Error>
    where T: Decode + Send + 'static
{
    let stream = Arc::clone(stream);
    let handle = task::spawn_blocking(move || {
        let mut stream_guard = stream.lock().unwrap();
        let stream = &mut *stream_guard;
        let mut bridge = SyncIoBridge::new(stream);
        let reader = IoExactReader::new(&mut bridge);
        let mut decoder = DecoderImpl::new(reader, config::legacy());
        let value = T::decode(&mut decoder)?;
        Ok(value)
    });
    handle.await?
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

#[cfg(test)]
mod tests {
    use bincode::config;
    use crate::PacketType;

    use super::Packet;

    #[test]
    fn test_decode() {
        let data: Vec<u8> = vec![0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x70, 0x61, 0x73, 0x73, 0x77, 0x72, 0x64, 0x00, 0x00];
        let (data, _): (Packet, usize) = bincode::decode_from_slice(&data, config::legacy()).unwrap();
        println!("{:?}", data);
        assert_eq!(data.id, 0);
        assert_eq!(data.r#type, PacketType::Auth);
        assert_eq!(data.body.to_str().unwrap(), "passwrd");
    }

    #[test]
    fn test_encode() {
        let packet = Packet::new(0, PacketType::Auth, "passwrd".into()).unwrap();
        let data = bincode::encode_to_vec(packet, config::legacy()).unwrap();
        assert_eq!(data, vec![0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x70, 0x61, 0x73, 0x73, 0x77, 0x72, 0x64, 0x00, 0x00]);
    }
}
