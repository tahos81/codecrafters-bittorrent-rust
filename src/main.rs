#![warn(clippy::pedantic)]

mod bencode;
mod peer;
mod torrent;
mod tracker;

use anyhow::Result;
use bencode::BencodeValue;
use bittorrent_starter_rust::mini_serde_bencode;
use bittorrent_starter_rust::mini_serde_bencode::from_bytes;
use mini_serde_bencode::from_str;
use tracker::discover_peers;

use std::env;
use std::fs;
use torrent::Torrent;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        let encoded_value = &args[2];
        let bencode_value = from_str::<BencodeValue>(encoded_value)?;
        println!("{bencode_value}");
    } else if command == "info" {
        let file = &args[2];
        let content = fs::read(file)?;
        let torrent = from_bytes::<Torrent>(&content)?;
        let info_hash = torrent.info_hash();

        println!("Tracker URL: {}", torrent.announce);
        println!("Length: {}", torrent.info.length);
        println!("Info Hash: {}", hex::encode(info_hash));
        println!("Piece Length: {}", torrent.info.piece_length);
        println!("Piece Hashes:");
        for piece in torrent.info.pieces.chunks(20) {
            println!("{}", hex::encode(piece));
        }
    } else if command == "peers" {
        let file = &args[2];
        let content = fs::read(file)?;
        let torrent = from_bytes::<Torrent>(&content)?;
        let resp = discover_peers(&torrent).await?;
        resp.iter().for_each(|peer| println!("{}", peer.to_url()));
    } else if command == "handshake" {
        let file = &args[2];
        let content = fs::read(file)?;
        let torrent = from_bytes::<Torrent>(&content)?;
        let resp = discover_peers(&torrent).await?;
        let peer = resp.iter().next().unwrap();
        let _peer = peer.connect(torrent.info_hash()).await?;
    } else {
        println!("unknown command: {}", args[1]);
    }

    Ok(())
}
