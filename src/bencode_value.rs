use std::fmt::Display;

use crate::dict::Dict;

#[derive(Debug)]
pub enum BencodeValue {
    String(String),
    Integer(i64),
    List(Vec<BencodeValue>),
    Dictionary(Dict),
}

impl Display for BencodeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BencodeValue::String(s) => write!(f, "\"{}\"", s),
            BencodeValue::Integer(i) => write!(f, "{}", i),
            BencodeValue::List(l) => {
                write!(f, "[")?;
                for (i, value) in l.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "{}", value)?;
                }
                write!(f, "]")
            }
            BencodeValue::Dictionary(d) => {
                write!(f, "{}", '{')?;
                for (i, (key, value)) in d.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "{}", key)?;
                    write!(f, ":")?;
                    write!(f, "{}", value)?;
                }
                write!(f, "{}", '}')
            }
        }
    }
}
