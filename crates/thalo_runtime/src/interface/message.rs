use std::time::Duration;

use anyhow::Result;
use bytes::Bytes;
use quinn::RecvStream;
use serde::{Deserialize, Serialize};
use thalo_message_store::GenericMessage;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Request {
    Execute {
        name: String,
        id: String,
        command: String,
        payload: String,
        timeout: Option<Duration>,
    },
    Publish {
        name: String,
        timeout: Option<Duration>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Response {
    Executed(ExecutedResult),
    Published,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutedResult {
    Events(Vec<GenericMessage<'static>>),
    TimedOut,
}

/// Packs a response to be sent over the network.
pub fn pack<T>(data: &T) -> Result<[Bytes; 2]>
where
    T: Serialize,
{
    let data = rmp_serde::to_vec_named(data)?;
    pack_raw(data)
}

pub fn pack_raw(data: Vec<u8>) -> Result<[Bytes; 2]> {
    let size = (data.len() as u32).to_le_bytes();
    let size: Bytes = Bytes::copy_from_slice(&size[..]);
    let bytes: Bytes = data.into();
    Ok([size, bytes])
}

pub async fn receive<T>(recv: &mut RecvStream) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let buffer = receive_raw(recv).await?;
    Ok(rmp_serde::from_slice(&buffer)?)
}

pub async fn receive_raw(recv: &mut RecvStream) -> Result<Vec<u8>> {
    let mut size = [0u8; 4];
    recv.read_exact(&mut size).await?;
    let size = u32::from_le_bytes(size);
    let mut buffer = vec![0u8; size as usize];
    recv.read_exact(&mut buffer).await?;
    Ok(buffer)
}
