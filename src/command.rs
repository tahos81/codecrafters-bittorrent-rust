use crate::{
    peer::{ConnectedPeer, Peer, PeerMessage},
    torrent::Torrent,
    tracker::discover_peers,
};
use anyhow::{anyhow, Result};
use bittorrent_starter_rust::{bitmap::BitMap, mini_serde_bencode::from_bytes};
use std::{
    fs::{self, File},
    io::Write,
    sync::Arc,
};
use tokio::sync::Mutex;

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
    let peer = peers.first().ok_or(anyhow!("No peers found"))?;
    let torrent = Arc::new(torrent);
    let peer = peer.connect(torrent).await?;
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
    let torrent = Arc::new(torrent);
    let mut connected_peers = connect_to_peers(peers, torrent.clone()).await?;
    let peer = connected_peers
        .first_mut()
        .ok_or(anyhow!("No peers found"))?;

    let piece = peer.download_piece(piece_index).await?;

    write_piece(&piece, output_file)?;
    println!("Piece {piece_index} downloaded to {output_file}");

    Ok(())
}

pub async fn download(output_file: &str, torrent_file: &str) -> Result<()> {
    let torrent = parse_torrent(torrent_file)?;
    let peers = discover_peers(&torrent).await?;
    let torrent = Arc::new(torrent);

    let piece_count = torrent.info.pieces.len() / 20;
    let piece_len = torrent.info.piece_length as usize;
    let pieces: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(vec![0; torrent.info.length as usize]));

    let connected_peers = connect_to_peers(peers, torrent).await?;
    let piece_idxs = Arc::new(Mutex::new((0..piece_count).collect::<Vec<usize>>()));
    let mut tasks = vec![];

    for (peer_idx, mut peer) in connected_peers.into_iter().enumerate() {
        let piece_idxs = Arc::clone(&piece_idxs);
        let pieces = Arc::clone(&pieces);
        let task = tokio::spawn(async move {
            loop {
                let mut lock = piece_idxs.lock().await;
                println!("Peer {peer_idx} has the lock");
                if let Some(piece_index) = lock.pop() {
                    drop(lock);
                    println!(
                        "Peer {peer_idx} dropped the lock and is downloading piece {piece_index}"
                    );
                    if let Ok(piece) = peer.download_piece(piece_index as u32).await {
                        println!("Peer {peer_idx} downloaded piece {piece_index}");
                        let start = piece_index * piece_len;
                        let end = start + piece.len();
                        let mut pieces = pieces.lock().await;
                        pieces.splice(start..end, piece);
                    } else {
                        {
                            let mut lock = piece_idxs.lock().await;
                            println!("Peer {peer_idx} failed to download piece {piece_index}");
                            lock.push(piece_index);
                        }
                    }
                } else {
                    break;
                }
            }
        });
        tasks.push(task);
    }

    for task in tasks {
        task.await?;
    }

    let pieces = pieces.lock().await;
    write_piece(&pieces, output_file)?;

    Ok(())
}

fn parse_torrent(torrent_file: &str) -> Result<Torrent> {
    let content = fs::read(torrent_file)?;
    let torrent = from_bytes::<Torrent>(&content)?;
    Ok(torrent)
}

fn write_piece(piece: &[u8], output_file: &str) -> Result<()> {
    let mut tmp_file = File::create(output_file)?;
    tmp_file.write_all(piece)?;
    Ok(())
}

async fn connect_to_peers(peers: Vec<Peer>, torrent: Arc<Torrent>) -> Result<Vec<ConnectedPeer>> {
    let mut connected_peers = Vec::new();
    let mut tasks = Vec::new();
    for (idx, peer) in peers.into_iter().enumerate() {
        let torrent = torrent.clone();
        let connect_task = tokio::spawn(async move {
            if let Ok(mut peer) = peer.connect(torrent).await {
                if let Ok(PeerMessage::Bitfield(bitfield)) = peer.receive_message().await {
                    let bitmap = BitMap::from(bitfield);
                    println!("BitMap for peer {idx}: {bitmap}");
                } else {
                    println!("Expected bitfield message");
                    return None;
                }

                if let Err(e) = peer.send_message(PeerMessage::Interested).await {
                    println!("Failed to send interested message: {e}");
                    return None;
                }
                peer.connection_state.set_am_interested(true);

                if let Ok(PeerMessage::Unchoke) = peer.receive_message().await {
                    peer.connection_state.set_peer_choking(false);
                    println!("Unchoked by peer {idx}");
                } else {
                    println!("Expected unchoke message");
                    return None;
                }

                return Some(peer);
            } else {
                println!("Failed to connect to peer {peer}");
                return None;
            }
        });

        tasks.push(connect_task);
    }

    for task in tasks {
        if let Some(peer) = task.await? {
            connected_peers.push(peer);
        }
    }

    if connected_peers.is_empty() {
        Err(anyhow!("Failed to connect to any peers"))?;
    }

    Ok(connected_peers)
}
