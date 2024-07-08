use std::{
    cmp::min,
    fs::{self, File},
    io::Write,
};

use anyhow::{anyhow, Result};
use bittorrent_starter_rust::{bitmap::BitMap, mini_serde_bencode::from_bytes};
use sha1::{Digest, Sha1};

use crate::{
    peer::{ConnectedPeer, Peer, PeerMessage},
    torrent::Torrent,
    tracker::discover_peers,
};

pub fn info(torrent_file: &str) -> Result<()> {
    let torrent = parse_torrent(torrent_file)?;
    let info_hash = torrent.info_hash();

    println!("Tracker URL: {}", torrent.announce);
    println!("Length: {}", torrent.info.length);
    println!("Info Hash: {}", hex::encode(info_hash));
    println!("Piece Length: {}", torrent.info.piece_length);
    println!("Piece Hashes:");
    for piece in torrent.info.pieces.chunks(20) {
        println!("{}", hex::encode(piece));
    }

    Ok(())
}

pub async fn peers(torrent_file: &str) -> Result<()> {
    let torrent = parse_torrent(torrent_file)?;
    let peers = discover_peers(&torrent).await?;
    peers.iter().for_each(|peer| println!("{}", peer.to_url()));

    Ok(())
}

pub async fn handshake(torrent_file: &str) -> Result<()> {
    let torrent = parse_torrent(torrent_file)?;
    let peers = discover_peers(&torrent).await?;
    let peer = peers.get(0).ok_or(anyhow!("No peers found"))?;
    let peer = peer.connect(torrent.info_hash()).await?;
    println!("Peer ID: {}", peer.peer_id);

    Ok(())
}

pub async fn download_and_write_piece(
    output_file: &str,
    torrent_file: &str,
    piece_index: u32,
) -> Result<()> {
    let torrent = parse_torrent(torrent_file)?;
    let peers = discover_peers(&torrent).await?;
    let mut connected_peers = connect_to_peers(peers, &torrent.info_hash()).await?;

    let piece = download_piece(&mut connected_peers, &torrent, piece_index).await?;

    write_piece(&piece, output_file)?;
    println!("Piece {piece_index} downloaded to {output_file}");

    Ok(())
}

pub async fn download(output_file: &str, torrent_file: &str) -> Result<()> {
    let torrent = parse_torrent(torrent_file)?;
    let peers = discover_peers(&torrent).await?;
    let mut connected_peers = connect_to_peers(peers, &torrent.info_hash()).await?;
    let piece_count = torrent.info.pieces.len() / 20;
    let mut pieces: Vec<Vec<u8>> = Vec::with_capacity(piece_count);

    for i in 0..piece_count {
        let piece = download_piece(&mut connected_peers, &torrent, i as u32).await?;
        pieces.push(piece);
    }

    let pieces = pieces.concat();

    write_piece(&pieces, output_file)?;

    Ok(())
}

async fn download_piece(
    connected_peers: &mut Vec<ConnectedPeer>,
    torrent: &Torrent,
    piece_index: u32,
) -> Result<Vec<u8>> {
    println!("Downloading piece: {piece_index}");
    let file_length = torrent.info.length;
    let piece_length = min(
        file_length - piece_index * torrent.info.piece_length,
        torrent.info.piece_length,
    );

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

    let piece_index = piece_index as usize;
    let piece_hash = &torrent.info.pieces[piece_index * 20..(piece_index + 1) * 20];
    if !check_piece(&piece, piece_hash) {
        Err(anyhow!("Piece hash does not match"))?;
    }

    Ok(piece)
}

fn parse_torrent(torrent_file: &str) -> Result<Torrent> {
    let content = fs::read(&torrent_file)?;
    let torrent = from_bytes::<Torrent>(&content)?;
    Ok(torrent)
}

fn check_piece(piece: &[u8], piece_hash: &[u8]) -> bool {
    let mut hasher = Sha1::new();
    hasher.update(piece);
    let hash = hasher.finalize().to_vec();
    hash == piece_hash
}

fn write_piece(piece: &[u8], output_file: &str) -> Result<()> {
    let mut tmp_file = File::create(&output_file)?;
    tmp_file.write_all(piece)?;
    Ok(())
}

async fn connect_to_peers(peers: Vec<Peer>, info_hash: &[u8]) -> Result<Vec<ConnectedPeer>> {
    let mut connected_peers = Vec::new();
    for peer in peers {
        if let Ok(mut peer) = peer.connect(info_hash.to_vec()).await {
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

    Ok(connected_peers)
}
