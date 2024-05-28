use std::collections::BTreeMap;
use std::str::from_utf8;

use anyhow::{bail, Result};

use crate::{bencode_value::BencodeValue, dict::Dict};

pub fn decode(value: &[u8]) -> Result<(BencodeValue, &[u8])> {
    match value.iter().next() {
        Some(b'i') => {
            let (decoded, remaining) = decode_integer(value)?;
            Ok((decoded, remaining))
        }
        Some(b'l') => {
            let (decoded, remaining) = decode_list(value)?;
            Ok((decoded, remaining))
        }
        Some(b'd') => {
            let (decoded, remaining) = decode_dict(value)?;
            Ok((decoded, remaining))
        }
        Some(_) => {
            let (decoded, remaining) = decode_string(value)?;
            Ok((decoded, remaining))
        }
        None => panic!("Empty data"),
    }
}

fn decode_string(value: &[u8]) -> Result<(BencodeValue, &[u8])> {
    // Example: "5:hello" -> "hello"
    let colon_index = value.iter().position(|&c| c == b':');
    match colon_index {
        None => bail!("Invalid string: {:?}", value),
        Some(colon_index) => {
            let len_str = from_utf8(&value[..colon_index])?;
            let len = len_str.parse::<usize>()?;
            let (start_idx, end_idx) = (colon_index + 1, colon_index + 1 + len);
            let remaining = &value[end_idx..];
            let try_str = from_utf8(&value[start_idx..end_idx]);
            match try_str {
                Err(_) => {
                    let decoded = hex::encode(&value[start_idx..end_idx]);
                    Ok((BencodeValue::String(decoded), remaining))
                }
                Ok(str) => {
                    let decoded = str.to_string();
                    Ok((BencodeValue::String(decoded), remaining))
                }
            }
        }
    }
}

fn decode_integer(value: &[u8]) -> Result<(BencodeValue, &[u8])> {
    // Example: "i42e" -> 42
    let end_idx = value.iter().position(|&c| c == b'e');
    match end_idx {
        None => bail!("Invalid integer: {:?}", value),
        Some(end_idx) => {
            let int_str = from_utf8(&value[1..end_idx])?;
            let int = int_str.parse::<i64>()?;
            let remaining = &value[end_idx + 1..];
            Ok((BencodeValue::Integer(int), remaining))
        }
    }
}

fn decode_list(value: &[u8]) -> Result<(BencodeValue, &[u8])> {
    // Example: "l5:helloi42ee" -> ["hello", 42]
    let mut list = Vec::new();
    let mut remaining = &value[1..];
    while remaining.iter().next() != Some(&b'e') {
        let (value, new_remaining) = decode(remaining)?;
        list.push(value);
        remaining = new_remaining;
    }
    Ok((BencodeValue::List(list), &remaining[1..]))
}

fn decode_dict(value: &[u8]) -> Result<(BencodeValue, &[u8])> {
    let mut dict: Dict = BTreeMap::new();
    let mut remaining = &value[1..];
    while remaining.iter().next() != Some(&b'e') {
        let (key, rem) = decode(remaining)?;
        let (value, new_remaining) = decode(rem)?;
        dict.insert(key.to_string(), value);
        remaining = new_remaining;
    }
    Ok((BencodeValue::Dictionary(dict), &remaining[1..]))
}
