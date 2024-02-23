
#![allow(async_fn_in_trait)]

use anyhow::Result;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub trait Encode {
    async fn encode(&self, encoder: &mut impl Encoder) -> Result<()>;
}

pub trait Encoder {
    async fn write(&mut self, bytes: &[u8]) -> Result<()>;
}

pub trait Decode: Sized + Send {
    async fn decode(decoder: &mut impl Decoder) -> Result<Self>;
}

pub trait Decoder {
    async fn read(&mut self, bytes: &mut [u8]) -> Result<()>;
}

pub struct IoWriter<T> {
    inner: T,
}

impl<T> IoWriter<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T: AsyncWrite + Unpin> Encoder for IoWriter<T> {
    async fn write(&mut self, bytes: &[u8]) -> Result<()> {
        self.inner.write_all(bytes).await?;
        Ok(())
    }
}

pub struct IoReader<T> {
    inner: T,
}

impl<T> IoReader<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T: AsyncRead + Unpin> Decoder for IoReader<T> {
    async fn read(&mut self, bytes: &mut [u8]) -> Result<()> {
        self.inner.read_exact(bytes).await?;
        Ok(())
    }
}

macro_rules! impl_coding_le_primitive {
    ($ty:ident, $bytes:expr) => {
        impl Encode for $ty {
            async fn encode(&self, encoder: &mut impl Encoder) -> Result<()> {
                encoder.write(&self.to_le_bytes()).await
            }
        }

        impl Decode for $ty {
            async fn decode(decoder: &mut impl Decoder) -> Result<Self> {
                let mut bytes = [0u8; $bytes];
                decoder.read(&mut bytes).await?;
                let value = $ty::from_le_bytes(bytes);
                Ok(value)
            }
        }
    };
}

impl_coding_le_primitive!(u8, 1);
impl_coding_le_primitive!(i16, 2);
impl_coding_le_primitive!(u16, 2);
impl_coding_le_primitive!(i32, 4);
impl_coding_le_primitive!(u32, 4);
impl_coding_le_primitive!(i64, 8);
impl_coding_le_primitive!(u64, 8);

pub async fn decode_from_slice<T: Decode>(slice: &[u8]) -> Result<T> {
    let mut slice: &[u8] = slice;
    let mut reader = IoReader::new(&mut slice);
    T::decode(&mut reader).await
}

pub async fn encode_to_vec<T: Encode>(value: &T) -> Result<Vec<u8>> {
    let mut buffer: Vec<u8> = vec![];
    let mut writer = IoWriter::new(&mut buffer);
    value.encode(&mut writer).await?;
    Ok(buffer)
}
