use anyhow::Result;
use bittorrent_starter_rust::mini_serde_bencode::from_bytes;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{peer::Peer, torrent::Torrent};

#[derive(Debug)]
struct TrackerRequest {
    info_hash: Vec<u8>,
    peer_id: String,
    port: u16,
    uploaded: u64,
    downloaded: u64,
    left: u64,
    compact: u8,
}

impl TrackerRequest {
    fn new(torrent: &Torrent) -> Self {
        Self {
            info_hash: torrent.info_hash(),
            peer_id: "00112233445566778899".to_string(),
            port: 6881,
            uploaded: 0,
            downloaded: 0,
            left: u64::from(torrent.info.length),
            compact: 1,
        }
    }

    fn to_url(&self, tracker: &str) -> String {
        let url = format!(
            "{}?info_hash={}&peer_id={}&port={}&uploaded={}&downloaded={}&left={}&compact={}",
            tracker,
            percent_encode(&self.info_hash),
            self.peer_id,
            self.port,
            self.uploaded,
            self.downloaded,
            self.left,
            self.compact
        );
        url
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct TrackerResponse {
    #[serde(default)]
    interval: u64,
    #[serde(with = "serde_bytes")]
    peers: Vec<u8>,
}

impl TrackerResponse {
    fn get_peers(&self) -> Vec<Peer> {
        let peers = self.peers.chunks(6).map(|chunk| {
            let ip = &chunk[..4];
            let port = u16::from_be_bytes([chunk[4], chunk[5]]);
            (ip, port)
        });
        let mut result = Vec::new();
        for (ip, port) in peers {
            let mut ip_array = [0; 4];
            ip_array.copy_from_slice(ip);
            result.push(Peer::new(ip_array, port));
        }
        result
    }
}

pub async fn discover_peers(torrent: &Torrent) -> Result<Vec<Peer>> {
    let tracker = torrent.announce.as_str();
    let client = Client::new();
    let request = TrackerRequest::new(torrent);
    let url = request.to_url(tracker);
    let response = client.get(&url).send().await?;
    let response: TrackerResponse = from_bytes(&response.bytes().await?)?;
    Ok(response.get_peers())
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
