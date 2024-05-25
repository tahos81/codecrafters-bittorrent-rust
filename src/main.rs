#![allow(dead_code)]

use anyhow::{bail, Result};
use std::{env, fmt::Display};

#[derive(Debug)]
enum BencodeValue {
    String(String),
    Integer(i64),
    List(Vec<BencodeValue>),
    Dictionary(Vec<(String, BencodeValue)>),
}

impl Display for BencodeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BencodeValue::String(s) => write!(f, "\"{}\"", s),
            BencodeValue::Integer(i) => write!(f, "{}", i),
            _ => write!(f, "Not implemented"),
        }
    }
}

fn decode_string(encoded_string: &str) -> Result<(BencodeValue, &str)> {
    // Example: "5:hello" -> "hello"
    let colon_index = encoded_string.find(':');
    match colon_index {
        None => bail!("Invalid string: {}", encoded_string),
        Some(colon_index) => {
            let number_string = &encoded_string[..colon_index];
            let number = number_string.parse::<i64>()?;
            let (start_idx, end_idx) = (colon_index + 1, colon_index + 1 + number as usize);
            let string = encoded_string[start_idx..end_idx].to_string();
            let remaining = &encoded_string[end_idx..];
            return Ok((BencodeValue::String(string), remaining));
        }
    }
}

fn decode_integer(encoded_integer: &str) -> Result<(BencodeValue, &str)> {
    // Example: "i42e" -> 42
    let end_idx = encoded_integer.find('e');
    match end_idx {
        None => bail!("Invalid integer: {}", encoded_integer),
        Some(end_idx) => {
            let number_string = &encoded_integer[1..end_idx];
            let number = number_string.parse::<i64>()?;
            let remaining = &encoded_integer[end_idx + 1..];
            return Ok((BencodeValue::Integer(number), remaining));
        }
    }
}

fn decode(encoded_value: &str) -> Result<BencodeValue> {
    match encoded_value.chars().next() {
        Some('i') => {
            let (value, _) = decode_integer(encoded_value)?;
            return Ok(value);
        }
        Some(_) => {
            let (value, _) = decode_string(encoded_value)?;
            return Ok(value);
        }
        None => panic!("Empty data"),
    }
}

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        let encoded_value = &args[2];
        let decoded_value = decode(encoded_value)?;
        println!("{}", decoded_value);
    } else {
        println!("unknown command: {}", args[1])
    }

    Ok(())
}
