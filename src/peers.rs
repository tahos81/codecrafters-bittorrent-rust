use anyhow::Result;
use bittorrent_starter_rust::mini_serde_bencode::from_bytes;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::torrent::Torrent;

#[derive(Debug, Deserialize, Serialize)]
pub struct TrackerResponse {
    interval: u64,
    #[serde(with = "serde_bytes")]
    peers: Vec<u8>,
}

impl TrackerResponse {
    pub fn print_peers(&self) {
        let peers = self.peers.chunks(6).map(|chunk| {
            let ip = &chunk[..4];
            let port = u16::from_be_bytes([chunk[4], chunk[5]]);
            (ip, port)
        });
        for (ip, port) in peers {
            println!(
                "{}:{}",
                ip.iter()
                    .map(|b| b.to_string())
                    .collect::<Vec<String>>()
                    .join("."),
                port
            );
        }
    }
}

pub async fn discover_peers(torrent: Torrent) -> Result<TrackerResponse> {
    let tracker = torrent.announce.as_str();
    let info_hash = torrent.info_hash();
    let client = Client::new();
    let query = &[
        ("peer_id", "00112233445566778899".to_string()),
        ("port", "6881".to_string()),
        ("uploaded", "0".to_string()),
        ("downloaded", "0".to_string()),
        ("left", torrent.info.length.to_string()),
        ("compact", "1".to_string()),
    ];
    let request = client.get(tracker).query(query).build()?;
    let mut url = request.url().to_string();
    url.push_str(&format!("&info_hash={}", percent_encode(&info_hash)));
    let response = client.get(&url).send().await?;
    let response: TrackerResponse = from_bytes(&response.bytes().await?)?;
    Ok(response)
}

fn percent_encode(input: &[u8]) -> String {
    let mut output = String::with_capacity(input.len() * 3);
    for &byte in input {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                output.push(byte as char);
            }
            byte => {
                output.push('%');
                output.push(hex_chars(byte >> 4));
                output.push(hex_chars(byte & 0x0F));
            }
        }
    }
    output
}

fn hex_chars(byte: u8) -> char {
    match byte {
        0..=9 => (byte + b'0') as char,
        10..=15 => (byte - 10 + b'A') as char,
        _ => unreachable!(),
    }
}
