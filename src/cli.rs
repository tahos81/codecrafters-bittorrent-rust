use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author = "Tahos81, tahirozpala@gmail.com", version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub cmd: Commands,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    Decode {
        value: String,
    },
    Info {
        torrent_file: String,
    },
    Peers {
        torrent_file: String,
    },
    Handshake {
        torrent_file: String,
        ip: Option<String>,
    },
    #[clap(name = "download_piece")]
    DownloadPiece {
        #[clap(short)]
        output_file: String,
        torrent_file: String,
        piece_index: u32,
    },
    Download {
        #[clap(short)]
        output_file: String,
        torrent_file: String,
    },
}
