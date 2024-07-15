use anyhow::{anyhow, Result};
use bittorrent_starter_rust::bitmap::BitMap;
use sha1::{Digest, Sha1};
use std::{cmp::min, fmt::Display, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::torrent::Torrent;

#[derive(Debug)]
pub enum PeerMessage {
    KeepAlive,
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have(u32),
    Bitfield(Vec<u8>),
    Request(u32, u32, u32),
    Piece(u32, u32, Vec<u8>),
    Cancel(u32, u32, u32),
}

pub struct ConnectionState {
    pub inner: BitMap,
}

impl ConnectionState {
    // 0: am_choking 1: am_interested 2: peer_choking 3: peer_interested
    fn new() -> Self {
        Self {
            inner: BitMap::from(vec![5]),
        }
    }

    #[allow(dead_code)]
    pub fn am_choking(&self) -> bool {
        !self.inner.get(0)
    }

    #[allow(dead_code)]
    pub fn am_interested(&self) -> bool {
        self.inner.get(1)
    }

    #[allow(dead_code)]
    pub fn peer_choking(&self) -> bool {
        self.inner.get(2)
    }

    #[allow(dead_code)]
    pub fn peer_interested(&self) -> bool {
        self.inner.get(3)
    }

    #[allow(dead_code)]
    pub fn set_am_choking(&mut self, value: bool) {
        if value {
            self.inner.unset(0);
        } else {
            self.inner.set(0);
        }
    }

    pub fn set_am_interested(&mut self, value: bool) {
        if value {
            self.inner.set(1);
        } else {
            self.inner.unset(1);
        }
    }

    pub fn set_peer_choking(&mut self, value: bool) {
        if value {
            self.inner.set(2);
        } else {
            self.inner.unset(2);
        }
    }

    #[allow(dead_code)]
    pub fn set_peer_interested(&mut self, value: bool) {
        if value {
            self.inner.set(3);
        } else {
            self.inner.unset(3);
        }
    }
}

pub struct ConnectedPeer {
    pub socket: TcpStream,
    pub peer_id: String,
    pub peer: Peer,
    pub connection_state: ConnectionState,
    pub torrent: Arc<Torrent>,
}

impl ConnectedPeer {
    fn new(socket: TcpStream, peer_id: String, peer: Peer, torrent: Arc<Torrent>) -> Self {
        Self {
            socket,
            peer_id,
            peer,
            connection_state: ConnectionState::new(),
            torrent,
        }
    }

    pub async fn send_message(&mut self, message: PeerMessage) -> Result<()> {
        let mut buf = Vec::new();
        match message {
            PeerMessage::KeepAlive => {}
            PeerMessage::Choke => buf.push(0),
            PeerMessage::Unchoke => buf.push(1),
            PeerMessage::Interested => buf.push(2),
            PeerMessage::NotInterested => buf.push(3),
            PeerMessage::Have(piece) => {
                buf.push(4);
                buf.extend(&piece.to_be_bytes());
            }
            PeerMessage::Bitfield(bitfield) => {
                buf.push(5);
                buf.extend(&bitfield);
            }
            PeerMessage::Request(index, begin, length) => {
                buf.push(6);
                buf.extend(&index.to_be_bytes());
                buf.extend(&begin.to_be_bytes());
                buf.extend(&length.to_be_bytes());
            }
            PeerMessage::Piece(index, begin, block) => {
                buf.push(7);
                buf.extend(&index.to_be_bytes());
                buf.extend(&begin.to_be_bytes());
                buf.extend(&block);
            }
            PeerMessage::Cancel(index, begin, length) => {
                buf.push(8);
                buf.extend(&index.to_be_bytes());
                buf.extend(&begin.to_be_bytes());
                buf.extend(&length.to_be_bytes());
            }
        }

        let len = buf.len() as u32;
        let len_buf = len.to_be_bytes();
        let mut message_buf = Vec::new();
        message_buf.extend(&len_buf);
        message_buf.extend(&buf);

        self.socket.write_all(&message_buf).await?;
        self.socket.flush().await?;
        Ok(())
    }

    pub async fn receive_message(&mut self) -> Result<PeerMessage> {
        let mut len_buf = [0; 4];

        self.socket.read_exact(&mut len_buf).await?;

        let len = u32::from_be_bytes(len_buf) as usize;
        if len == 0 {
            return Ok(PeerMessage::KeepAlive);
        }
        let mut id_buf = [0; 1];
        self.socket.read_exact(&mut id_buf).await?;
        let id = id_buf[0];
        let mut payload = vec![0; len - 1];
        self.socket.read_exact(&mut payload).await?;
        match id {
            0 => Ok(PeerMessage::Choke),
            1 => Ok(PeerMessage::Unchoke),
            2 => Ok(PeerMessage::Interested),
            3 => Ok(PeerMessage::NotInterested),
            4 => {
                let piece = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
                Ok(PeerMessage::Have(piece))
            }
            5 => Ok(PeerMessage::Bitfield(payload)),
            6 => {
                let index = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
                let begin = u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]);
                let length = u32::from_be_bytes([payload[8], payload[9], payload[10], payload[11]]);
                Ok(PeerMessage::Request(index, begin, length))
            }
            7 => {
                let index = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
                let begin = u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]);
                Ok(PeerMessage::Piece(index, begin, payload[8..].to_vec()))
            }
            8 => {
                let index = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
                let begin = u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]);
                let length = u32::from_be_bytes([payload[8], payload[9], payload[10], payload[11]]);
                Ok(PeerMessage::Cancel(index, begin, length))
            }
            _ => Err(anyhow!("Unknown message id: {}", id)),
        }
    }

    pub async fn download_piece(&mut self, piece_index: u32) -> Result<Vec<u8>> {
        println!("Downloading piece: {piece_index}");
        let file_length = self.torrent.info.length;
        let piece_length = min(
            file_length - piece_index * self.torrent.info.piece_length,
            self.torrent.info.piece_length,
        );

        let mut piece: Vec<u8> = Vec::with_capacity(piece_length as usize);
        let block_size = 2u32.pow(14);
        let mut rem = piece_length;

        while rem > 0 {
            let size = min(rem, block_size);

            self.send_message(PeerMessage::Request(piece_index, piece_length - rem, size))
                .await?;

            if let PeerMessage::Piece(piece_idx, begin, block) = self.receive_message().await? {
                println!("Received block: {begin} for piece {piece_idx}");
                piece.extend_from_slice(&block);
                rem -= size;
            } else {
                Err(anyhow!("Failed to download piece"))?;
            }
        }

        let piece_index = piece_index as usize;
        let piece_hash = &self.torrent.info.pieces[piece_index * 20..(piece_index + 1) * 20];
        if !check_piece(&piece, piece_hash) {
            Err(anyhow!("Piece hash does not match"))?;
        }

        Ok(piece)
    }
}

fn check_piece(piece: &[u8], piece_hash: &[u8]) -> bool {
    let mut hasher = Sha1::new();
    hasher.update(piece);
    let hash = hasher.finalize().to_vec();
    hash == piece_hash
}

#[derive(Debug, Clone, Copy)]
pub struct Peer {
    ip: [u8; 4],
    port: u16,
}

impl Display for Peer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}",
            self.ip
                .iter()
                .map(std::string::ToString::to_string)
                .collect::<Vec<String>>()
                .join("."),
            self.port
        )
    }
}

impl Peer {
    pub fn new(ip: [u8; 4], port: u16) -> Self {
        Self { ip, port }
    }

    pub fn to_url(self) -> String {
        self.to_string()
    }

    pub async fn connect(self, torrent: Arc<Torrent>) -> Result<ConnectedPeer> {
        if let Ok(mut socket) = TcpStream::connect(self.to_url()).await {
            let handshake = Handshake::new(torrent.info_hash(), "00112233445566778899".to_string());
            socket.write_all(&handshake.to_bytes()).await?;
            socket.flush().await?;
            let mut buf = [0; 68];
            socket.read_exact(&mut buf).await?;
            let handshake = Handshake::from_buf(buf)?;
            Ok(ConnectedPeer::new(socket, handshake.peer_id, self, torrent))
        } else {
            Err(anyhow!("Failed to connect to peer"))
        }
    }
}

struct Handshake {
    info_hash: Vec<u8>,
    peer_id: String,
}

impl Handshake {
    fn new(info_hash: Vec<u8>, peer_id: String) -> Self {
        Self { info_hash, peer_id }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(19);
        buf.extend(b"BitTorrent protocol");
        buf.extend(&[0; 8]);
        buf.extend(&self.info_hash);
        buf.extend(self.peer_id.as_bytes());
        buf
    }

    fn from_buf(buf: [u8; 68]) -> Result<Self> {
        if buf[0] != 19 {
            return Err(anyhow!("Invalid handshake"));
        }
        let info_hash = buf[28..48].to_vec();
        let peer_id = hex::encode(&buf[48..68]);
        Ok(Self { info_hash, peer_id })
    }
}
