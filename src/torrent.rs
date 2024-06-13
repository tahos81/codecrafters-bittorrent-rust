use bittorrent_starter_rust::mini_serde_bencode::to_bytes;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

#[derive(Debug, Serialize, Deserialize)]
pub struct Torrent {
    pub announce: String,
    pub info: Info,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Info {
    pub length: i64,
    pub name: String,
    #[serde(rename = "piece length")]
    pub piece_length: i64,
    #[serde(with = "serde_bytes")]
    pub pieces: Vec<u8>,
}

impl Torrent {
    pub fn info_hash(&self) -> Vec<u8> {
        let info = to_bytes(&self.info).unwrap();

        let mut hasher = Sha1::new();
        hasher.update(&info);
        hasher.finalize().to_vec()
    }
}

////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mini_serde_bencode::from_str;

    #[test]
    fn test_de_torrent() {
        let input = "d8:announce39:http://torrent.ubuntu.com:6969/announce4:infod6:lengthi282334976e4:name28:ubuntu-20.04.1-desktop-amd6412:piece lengthi20e6:pieces22:0123456789abcdef012345ee";
        let torrent = from_str::<Torrent>(input).unwrap();
        assert_eq!(torrent.announce, "http://torrent.ubuntu.com:6969/announce");
        assert_eq!(torrent.info.name, "ubuntu-20.04.1-desktop-amd64");
        assert_eq!(torrent.info.piece_length, 20);
        assert_eq!(torrent.info.pieces.len(), 22);
    }

    #[test]
    fn test_de_info() {
        let input = "d6:lengthi282334976e4:name28:ubuntu-20.04.1-desktop-amd6412:piece lengthi20e6:pieces22:0123456789abcdef012345e";
        let info = from_str::<Info>(input).unwrap();
        assert_eq!(info.length, 282334976);
        assert_eq!(info.name, "ubuntu-20.04.1-desktop-amd64");
        assert_eq!(info.piece_length, 20);
        assert_eq!(info.pieces.len(), 22);
    }
}
