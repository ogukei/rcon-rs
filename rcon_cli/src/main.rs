

use std::env;

use anyhow::{bail, Result};
use rcon::{client::RconClient, Packet, PacketType};
use log::trace;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    // connect
    let endpoint = env::var("RCON_ENDPOINT").expect("RCON_ENDPOINT is required");
    trace!("connecting");
    let client = RconClient::connect(&endpoint).await?;
    trace!("connected");
    // auth
    const AUTH_PACKET_ID: i32 = 0;
    let password = env::var("RCON_PASSWORD").expect("RCON_PASSWORD is required");
    let auth_request = Packet::new(AUTH_PACKET_ID, PacketType::AUTH, password.into())?;
    trace!("sending auth request");
    client.write_packet(auth_request).await?;
    // await next auth response
    trace!("awaiting auth reponse");
    let auth_response = loop {
        let packet = client.read_packet().await?;
        if packet.r#type() == PacketType::AUTH_RESPONSE {
            break packet
        }
    };
    // check auth result
    if auth_response.id() == AUTH_PACKET_ID {
        trace!("authentication successful");
    } else {
        bail!("authentication failure");
    }
    // command
    const COMMAND_PACKET_ID: i32 = 1;
    let command = env::var("RCON_COMMAND").expect("RCON_COMMAND is required");
    let command_request = Packet::new(COMMAND_PACKET_ID, PacketType::EXEC_COMMAND, command)?;
    trace!("sending command request");
    client.write_packet(command_request).await?;
    // awaiting command response
    trace!("awaiting command reponse");
    let response_value = loop {
        let packet = client.read_packet().await?;
        if packet.id() == COMMAND_PACKET_ID && packet.r#type() == PacketType::RESPONSE_VALUE {
            break packet
        }
    };
    let response_body = response_value.body()?;
    trace!("command response acquired");
    println!("{}", response_body);
    Ok(())
}
