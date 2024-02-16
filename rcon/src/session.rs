
use std::sync::{Arc, Mutex};
use bincode::{config, de::{read::Reader, DecoderImpl}, enc::{write::Writer, EncoderImpl}, error::{DecodeError, EncodeError}, Decode, Encode};

use tokio::net::TcpStream;
use tokio_util::io::SyncIoBridge;
use tokio::task;

use anyhow::Result;

use crate::packet::Packet;
use crate::client::RconClient;

pub struct RconSession {
    client: Arc<RconClient>,
}

impl RconSession {
    pub async fn new(client: &Arc<RconClient>) -> Result<Self> {
        let client = Arc::clone(client);
        let this = Self {
            client,
        };
        Ok(this)
    }

    async fn run(&self) {
        let stream = self.client.stream();
        loop {
            tokio::select! {
                packet = self.client.read_packet(), if stream.lock().unwrap().readable().await.is_ok() => {
                    
                },
                
            }
        }
    }
}
