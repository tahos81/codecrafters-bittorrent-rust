use std::collections::BTreeMap;

use crate::bencode_value::BencodeValue;

pub type Dict = BTreeMap<String, BencodeValue>;
