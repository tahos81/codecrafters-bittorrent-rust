use crate::mini_serde_bencode::error::{Error, Result};
use serde::{
    de::{self, EnumAccess, VariantAccess},
    forward_to_deserialize_any, Deserialize,
};
use std::ops::{AddAssign, MulAssign, Neg};

pub struct Deserializer<'de> {
    input: &'de [u8],
}

impl<'de> Deserializer<'de> {
    pub fn from_str(input: &'de str) -> Self {
        Deserializer {
            input: input.as_bytes(),
        }
    }

    pub fn from_bytes(input: &'de [u8]) -> Self {
        Deserializer { input }
    }
}

pub fn from_str<'a, T>(s: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_str(s);
    let t = T::deserialize(&mut deserializer)?;
    if deserializer.input.is_empty() {
        Ok(t)
    } else {
        Err(Error::Message("trailing bytes".to_string()))
    }
}

pub fn from_bytes<'a, T>(b: &'a [u8]) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_bytes(b);
    let t = T::deserialize(&mut deserializer)?;
    if deserializer.input.is_empty() {
        Ok(t)
    } else {
        Err(Error::Message("trailing bytes".to_string()))
    }
}

impl<'de> Deserializer<'de> {
    // Look at the first character in the input without consuming it.
    fn peek_byte(&mut self) -> Result<u8> {
        self.input
            .iter()
            .next()
            .copied()
            .ok_or(Error::Message("eof".to_string()))
    }

    // Consume the first character in the input.
    fn next_byte(&mut self) -> Result<u8> {
        let by = self.peek_byte()?;
        self.input = &self.input[1..];
        Ok(by)
    }

    fn parse_signed<T>(&mut self) -> Result<T>
    where
        T: Neg<Output = T> + AddAssign<T> + MulAssign<T> + From<i8>,
    {
        match self.next_byte()? {
            b'i' => {}
            _ => {
                return Err(Error::Message("expected i".to_string()));
            }
        };

        let is_neg = matches!(self.peek_byte()?, b'-');

        if is_neg {
            self.next_byte()?;
        }

        let mut int = match self.next_byte()? {
            by @ b'0'..=b'9' => T::from((by - b'0') as i8),
            _ => {
                return Err(Error::Message("invalid int".to_string()));
            }
        };

        loop {
            match self.input.iter().next() {
                Some(by @ b'0'..=b'9') => {
                    self.input = &self.input[1..];
                    int *= T::from(10);
                    int += T::from((by - b'0') as i8);
                }
                Some(b'e') => {
                    self.input = &self.input[1..];
                    if is_neg {
                        int *= T::from(-1);
                        return Ok(int);
                    } else {
                        return Ok(int);
                    }
                }
                _ => {
                    return Err(Error::Message("expected e".to_string()));
                }
            }
        }
    }

    fn parse_bytes(&mut self) -> Result<Vec<u8>> {
        if let Some(colon_index) = self.input.iter().position(|b| b == &b':') {
            let len_str = std::str::from_utf8(&self.input[..colon_index])
                .map_err(|_| Error::Message("invalid utf8".to_string()))?;
            let len: usize = len_str
                .parse()
                .map_err(|_| Error::Message("invalid len".to_string()))?;
            let bytes = self.input[colon_index + 1..colon_index + 1 + len].to_vec();
            self.input = &self.input[colon_index + 1 + len..];
            Ok(bytes)
        } else {
            Err(Error::Message("expected :".to_string()))
        }
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.peek_byte()? {
            b'0'..=b'9' => self.deserialize_byte_buf(visitor),
            b'i' => self.deserialize_i64(visitor),
            b'l' => self.deserialize_seq(visitor),
            b'd' => self.deserialize_map(visitor),
            _ => Err(Error::Message("invalid type".to_string())),
        }
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes option struct tuple tuple_struct newtype_struct unit unit_struct
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_i64(self.parse_signed()?)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_byte_buf(self.parse_bytes()?)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        if self.next_byte()? == b'l' {
            // Give the visitor access to each element of the sequence.
            let value = visitor.visit_seq(Seq::new(self))?;

            // Parse the closing bracket of the sequence.
            if self.next_byte()? == b'e' {
                Ok(value)
            } else {
                Err(Error::Message("expected e".to_string()))
            }
        } else {
            Err(Error::Message("expected l".to_string()))
        }
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        if self.next_byte()? == b'd' {
            // Give the visitor access to each element of the sequence.
            let value = visitor.visit_map(Map::new(self))?;
            // Parse the closing bracket of the sequence.
            if self.next_byte()? == b'e' {
                Ok(value)
            } else {
                Err(Error::Message("expected e".to_string()))
            }
        } else {
            Err(Error::Message("expected d".to_string()))
        }
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        if self.next_byte()? == b'd' {
            // Visit a newtype variant, tuple variant, or struct variant.
            let value = visitor.visit_enum(Enum::new(self))?;
            // Parse the matching close brace.
            if self.next_byte()? == b'e' {
                Ok(value)
            } else {
                Err(Error::Message("expected e".to_string()))
            }
        } else {
            Err(Error::Message("expected d".to_string()))
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

impl<'de, 'a> de::SeqAccess<'de> for Seq<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: de::DeserializeSeed<'de>,
    {
        if self.de.peek_byte()? == b'e' {
            return Ok(None);
        }

        seed.deserialize(&mut *self.de).map(Some)
    }
}

impl<'de, 'a> de::MapAccess<'de> for Map<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: de::DeserializeSeed<'de>,
    {
        if self.de.peek_byte()? == b'e' {
            return Ok(None);
        }

        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: de::DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de)
    }
}

impl<'de, 'a> EnumAccess<'de> for Enum<'a, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: de::DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(&mut *self.de)?;
        Ok((variant, self))
    }
}

// `VariantAccess` is provided to the `Visitor` to give it the ability to see
// the content of the single variant that it decided to deserialize.
impl<'de, 'a> VariantAccess<'de> for Enum<'a, 'de> {
    type Error = Error;

    // If the `Visitor` expected this variant to be a unit variant, the input
    // should have been the plain string case handled in `deserialize_enum`.
    fn unit_variant(self) -> Result<()> {
        Err(Error::Message("expected unit variant".to_string()))
    }

    // Newtype variants are represented in JSON as `{ NAME: VALUE }` so
    // deserialize the value here.
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: de::DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de)
    }

    // Tuple variants are represented in JSON as `{ NAME: [DATA...] }` so
    // deserialize the sequence of data here.
    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        de::Deserializer::deserialize_seq(&mut *self.de, visitor)
    }

    // Struct variants are represented in JSON as `{ NAME: { K: V, ... } }` so
    // deserialize the inner map here.
    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        de::Deserializer::deserialize_map(&mut *self.de, visitor)
    }
}

struct Seq<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> Seq<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Seq { de }
    }
}

struct Map<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> Map<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Map { de }
    }
}

struct Enum<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> Enum<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Enum { de }
    }
}

////////////////////////////////////////////////////////////////////////////////
#[cfg(test)]
mod tests {
    use super::*;
    use serde_bytes::ByteBuf;

    #[test]
    fn test_int() {
        let j = "i5e";
        let expected = 5;
        assert_eq!(expected, from_str(j).unwrap());
    }

    #[test]
    fn test_negative_int() {
        let j = "i-5e";
        let expected = -5;
        assert_eq!(expected, from_str(j).unwrap());
    }

    #[test]
    fn test_str() {
        let j = "4:spam";
        let expected = "spam".as_bytes().to_vec();
        let actual: ByteBuf = from_str(j).unwrap();
        assert_eq!(expected, *actual);
    }

    #[test]
    fn test_list() {
        let j = "li5ei6ee";
        let expected = (5i64, 6i64);
        let actual = from_str(j).unwrap();
        assert_eq!(expected, actual);
    }
}
