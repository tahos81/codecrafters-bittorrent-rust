#![warn(clippy::pedantic)]

mod bencode;
mod torrent;

use anyhow::Result;
use bencode::BencodeValue;
use bittorrent_starter_rust::mini_serde_bencode;
use bittorrent_starter_rust::mini_serde_bencode::from_bytes;
use mini_serde_bencode::from_str;
use std::env;
use std::fs;
use torrent::Torrent;

fn main() -> Result<()> {
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
    } else {
        println!("unknown command: {}", args[1]);
    }

    Ok(())
}
