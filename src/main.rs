#![warn(clippy::pedantic)]

mod bencode;
mod peer;
mod torrent;
mod tracker;

use anyhow::anyhow;
use anyhow::Result;
use bencode::BencodeValue;
use bittorrent_starter_rust::bitmap::BitMap;
use bittorrent_starter_rust::mini_serde_bencode;
use bittorrent_starter_rust::mini_serde_bencode::from_bytes;
use mini_serde_bencode::from_str;
use peer::PeerMessage;
use sha1::{Digest, Sha1};
use std::cmp::min;
use std::env;
use std::fs;
use std::fs::File;
use std::io::Write;
use torrent::Torrent;
use tracker::discover_peers;

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
        let peer = resp.get(0).ok_or(anyhow::anyhow!("No peers found"))?;
        let peer = peer.connect(torrent.info_hash()).await?;
        println!("Peer ID: {}", peer.peer_id);
    } else if command == "download_piece" {
        let file = &args[4];
        let content = fs::read(file)?;
        let torrent = from_bytes::<Torrent>(&content)?;
        let peers = discover_peers(&torrent).await?;
        let mut connected_peers = Vec::new();
        for peer in peers {
            if let Ok(mut peer) = peer.connect(torrent.info_hash()).await {
                if let PeerMessage::Bitfield(bitfield) = peer.receive_message().await? {
                    let bitmap = BitMap::from(bitfield);
                    println!("BitMap: {bitmap}");
                } else {
                    println!("Expected bitfield message");
                    continue;
                }
                if let Err(e) = peer.send_message(PeerMessage::Interested).await {
                    println!("Failed to send interested message: {e}");
                    continue;
                }
                peer.connection_state.set_am_interested(true);
                if let PeerMessage::Unchoke = peer.receive_message().await? {
                    peer.connection_state.set_peer_choking(false);
                    println!("Unchoked");
                } else {
                    println!("Expected unchoke message");
                    continue;
                }
                connected_peers.push(peer);
            } else {
                println!("Failed to connect to peer {peer}");
            }
        }

        if connected_peers.is_empty() {
            Err(anyhow!("Failed to connect to any peers"))?;
        }

        let file_length = torrent.info.length as u32;
        println!("File length: {file_length}");
        let piece_length = torrent.info.piece_length as u32;
        println!("Piece length: {piece_length}");
        let piece_index: u32 = args[5].parse()?;
        println!("Downloading piece: {piece_index}");
        let piece_length = min(file_length - piece_index * piece_length, piece_length);
        println!("Actual Piece length: {piece_length}");

        let mut piece: Vec<u8> = Vec::with_capacity(piece_length as usize);
        let block_size = 2u32.pow(14);
        let mut rem = piece_length;
        let mut peer_idx = 0;

        while rem > 0 {
            let peer = &mut connected_peers[peer_idx];
            let size = min(rem, block_size);

            println!("Requesting block: {} {}", piece_length - rem, size);
            peer.send_message(PeerMessage::Request(piece_index, piece_length - rem, size))
                .await?;

            if let PeerMessage::Piece(_, begin, block) = peer.receive_message().await? {
                println!("Received block: {} {}", begin, block.len());
                piece.extend_from_slice(&block);
                rem -= size;
            } else {
                println!("Expected piece message");
                peer_idx = peer_idx + 1;
                if peer_idx == connected_peers.len() {
                    Err(anyhow!("Failed to download piece"))?;
                }
            }
        }

        let mut hasher = Sha1::new();
        hasher.update(&piece);
        let piece_hash = hasher.finalize().to_vec();
        let piece_index = piece_index as usize;
        let expected_hash = &torrent.info.pieces[piece_index * 20..(piece_index + 1) * 20];
        if piece_hash != expected_hash {
            Err(anyhow!("Piece hash does not match"))?;
        }

        let path = &args[3];
        let mut tmp_file = File::create(path)?;
        tmp_file.write_all(&piece)?;
        println!("Piece {piece_index} downloaded to {path}");
    } else {
        println!("unknown command: {}", args[1]);
    }

    Ok(())
}
