use anyhow::{anyhow, Result};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

pub struct ConnectedPeer {
    socket: TcpStream,
    peer_id: String,
    peer: Peer,
}

#[derive(Debug, Clone, Copy)]
pub struct Peer {
    ip: [u8; 4],
    port: u16,
}

impl Peer {
    pub fn new(ip: [u8; 4], port: u16) -> Self {
        Self { ip, port }
    }

    pub fn to_url(&self) -> String {
        format!(
            "{}:{}",
            self.ip
                .iter()
                .map(|byte| byte.to_string())
                .collect::<Vec<String>>()
                .join("."),
            self.port
        )
    }

    pub async fn connect(self, info_hash: Vec<u8>) -> Result<ConnectedPeer> {
        let peer_handle = tokio::spawn(async move {
            if let Ok(mut socket) = TcpStream::connect(self.to_url()).await {
                let handshake = Handshake::new(info_hash, "00112233445566778899".to_string());
                socket.write_all(&handshake.to_bytes()).await?;
                socket.flush().await?;
                let mut buf = [0; 68];
                socket.read_exact(&mut buf).await?;
                let handshake = Handshake::from_buf(buf)?;
                println!("Peer ID: {}", handshake.peer_id);
                Ok(ConnectedPeer {
                    socket,
                    peer_id: handshake.peer_id,
                    peer: self,
                })
            } else {
                Err(anyhow!("Failed to connect to peer"))
            }
        });

        peer_handle.await?
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
        let peer_id = hex::encode(buf[48..68].to_vec());
        Ok(Self { info_hash, peer_id })
    }
}
