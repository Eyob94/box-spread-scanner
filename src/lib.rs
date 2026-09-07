use std::collections::HashMap;

use serde::Serialize;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        TcpStream,
        tcp::{OwnedReadHalf, OwnedWriteHalf},
    },
};
use tracing::{debug, info, instrument};

use crate::message::{IBMessage, OptionSide, parse_message};

mod actions;
mod boxspread;
mod config;
mod data;
mod message;
mod server;

pub use config::*;
pub use server::*;

#[derive(Debug, Default, Clone, Serialize)]
pub struct OptionChainParams {
    pub underlying_con_id: u32,
    pub multiplier: u32,
    pub expirations: Vec<chrono::NaiveDate>,
    pub strikes: Vec<u32>,
    pub quotes: HashMap<(u32, OptionSide), OptionQuote>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct OptionQuote {
    pub bid: Option<u32>,
    pub ask: Option<u32>,
    pub bid_size: u64,
    pub ask_size: u64,
    pub delta: Option<f64>,
}

impl OptionQuote {
    pub fn complete(&self) -> bool {
        self.bid.is_some()
            && self.ask.is_some()
            && self.bid_size > 0
            && self.ask_size > 0
            // && self.delta.is_some()
    }
}

#[derive(Default, Serialize)]
pub struct IBData {
    pub handshake: Option<bool>,
    pub start_api: Option<bool>,
    pub spx_options_chains: HashMap<(String, String), OptionChainParams>,
    pub spx_spot_price: Option<u32>,
}

#[instrument]
pub async fn start_connection(
    port: u16,
    client_id: u16,
) -> eyre::Result<(OwnedReadHalf, OwnedWriteHalf, IBData)> {
    let mut data = IBData::default();

    let (mut reader, mut writer) = TcpStream::connect(format!("127.0.0.1:{port}"))
        .await?
        .into_split();

    writer.write_all(&IBMessage::handshake()).await?;

    data.handshake = Some(false);

    let handshake_payload = read_message_from_ibkr(&mut reader).await?;

    let handshake_msg = parse_message(&handshake_payload);

    info!(?handshake_msg, "Handshake complete");

    data.handshake = Some(true);

    send_message_to_ibkr(&mut writer, IBMessage::start_api_bytes(client_id)).await?;

    data.start_api = Some(false);

    loop {
        let payload = read_message_from_ibkr(&mut reader).await?;
        let fields = parse_message(&payload)?;
        info!(?fields, "API searching");

        if fields.first() == Some(&"9") {
            info!("API is accepted :D");
            data.start_api = Some(true);
            break;
        }
    }

    Ok((reader, writer, data))
}

pub async fn read_message_from_ibkr(
    reader: &mut tokio::net::tcp::OwnedReadHalf,
) -> eyre::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf).await?;
    let len = i32::from_be_bytes(len_buf) as usize;

    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload).await?;

    debug!(len, "Message read");
    Ok(payload)
}

pub async fn send_message_to_ibkr(
    writer: &mut OwnedWriteHalf,
    payload: Vec<u8>,
) -> eyre::Result<()> {
    let len = payload.len() as u32;
    let byte_len = len.to_be_bytes();

    writer.write_all(&byte_len).await?;

    writer.write_all(&payload).await?;
    info!(len, "Message sent");

    Ok(())
}
