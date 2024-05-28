#![warn(clippy::pedantic)]

mod bencode_value;
mod decoder;
mod dict;

use anyhow::bail;
use anyhow::Result;
use bencode_value::BencodeValue;
use std::env;
use std::fs;

use crate::decoder::decode;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        let encoded_value = &args[2];
        let (decoded_value, _) = decode(encoded_value.as_bytes())?;
        println!("{}", decoded_value);
    } else if command == "info" {
        let file = &args[2];
        let content = fs::read(file)?;
        let (decoded_value, _) = decode(&content)?;
        match decoded_value {
            BencodeValue::Dictionary(dict) => {
                if let Some(BencodeValue::String(announce)) = dict.get("\"announce\"") {
                    print!("Tracker URL: {}", announce);
                }

                if let Some(BencodeValue::Dictionary(dict)) = dict.get("\"info\"") {
                    if let Some(length) = dict.get("\"length\"") {
                        print!("Length: {}", length);
                    }
                }
            }
            _ => {
                bail!("invalid torrent file")
            }
        }
    } else {
        println!("unknown command: {}", args[1]);
    }

    Ok(())
}
