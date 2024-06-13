use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use std::{collections::BTreeMap, fmt::Display};

type Dict = BTreeMap<ByteBuf, BencodeValue>;

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(untagged)]
pub enum BencodeValue {
    List(Vec<BencodeValue>),
    #[serde(with = "serde_bytes")]
    String(Vec<u8>),
    Integer(i64),
    Dictionary(Dict),
}

impl Display for BencodeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BencodeValue::String(s) => {
                let s = String::from_utf8_lossy(s);
                write!(f, "\"{s}\"")
            }
            BencodeValue::Integer(i) => write!(f, "{i}"),
            BencodeValue::List(l) => {
                write!(f, "[")?;
                for (i, value) in l.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "{value}")?;
                }
                write!(f, "]")
            }
            BencodeValue::Dictionary(d) => {
                write!(f, "{{")?;
                for (i, (key, value)) in d.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    let key = String::from_utf8_lossy(key);
                    write!(f, "\"{key}\"")?;
                    write!(f, ":")?;
                    write!(f, "{value}")?;
                }
                write!(f, "}}")
            }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use serde_bytes::ByteBuf;

    use crate::bencode::Dict;
    use crate::mini_serde_bencode::{from_str, to_string};
    use crate::BencodeValue;

    #[test]
    fn test_ser() {
        let s = BencodeValue::String(hex::decode("68656c6c6f").unwrap());
        let expected = "5:hello";
        assert_eq!(to_string(&s).unwrap(), expected);

        let i = BencodeValue::Integer(5);
        let expected = "i5e";
        assert_eq!(to_string(&i).unwrap(), expected);

        let l = BencodeValue::List(vec![
            BencodeValue::Integer(5),
            BencodeValue::String(hex::decode("68656c6c6f").unwrap()),
        ]);
        let expected = "li5e5:helloe";
        assert_eq!(to_string(&l).unwrap(), expected);

        let mut dict = Dict::new();
        dict.insert(
            ByteBuf::from(hex::decode("68656c6c6f").unwrap()),
            BencodeValue::Integer(5),
        );
        let d = BencodeValue::Dictionary(dict);
        let expected = "d5:helloi5ee";
        assert_eq!(to_string(&d).unwrap(), expected);
    }

    #[test]
    fn test_de() {
        let s = from_str::<BencodeValue>("5:tahir").unwrap();
        let expected = BencodeValue::String(hex::decode("7461686972").unwrap());
        assert_eq!(s, expected);

        let i = from_str::<BencodeValue>("i5e").unwrap();
        let expected = BencodeValue::Integer(5);
        assert_eq!(i, expected);

        let l = from_str::<BencodeValue>("le").unwrap();
        let expected = BencodeValue::List(vec![]);
        assert_eq!(l, expected);

        let l = from_str::<BencodeValue>("li5e5:helloe").unwrap();
        let expected = BencodeValue::List(vec![
            BencodeValue::Integer(5),
            BencodeValue::String(hex::decode("68656c6c6f").unwrap()),
        ]);
        assert_eq!(l, expected);

        let d = from_str::<BencodeValue>("d5:helloi5ee").unwrap();
        let mut dict = Dict::new();
        dict.insert(
            ByteBuf::from(hex::decode("68656c6c6f").unwrap()),
            BencodeValue::Integer(5),
        );
        let expected = BencodeValue::Dictionary(dict);
        assert_eq!(d, expected);
    }
}
