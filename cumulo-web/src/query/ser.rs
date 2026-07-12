use leptos_router::params::ParamsMap;
use serde::ser::{Error as _, Impossible, SerializeMap, SerializeStruct};
use serde::{Serialize, Serializer};

use super::error::Error;

pub fn to_1nest_params<T: Serialize>(value: &T) -> Result<ParamsMap, Error> {
    value.serialize(PairsSerializer)
}

struct PairsSerializer;

impl Serializer for PairsSerializer {
    type Ok = ParamsMap;
    type Error = Error;
    type SerializeMap = TopBuilder;
    type SerializeStruct = TopBuilder;
    type SerializeSeq = Impossible<Self::Ok, Error>;
    type SerializeTuple = Impossible<Self::Ok, Error>;
    type SerializeTupleStruct = Impossible<Self::Ok, Error>;
    type SerializeTupleVariant = Impossible<Self::Ok, Error>;
    type SerializeStructVariant = Impossible<Self::Ok, Error>;

    fn serialize_map(self, _len: Option<usize>) -> Result<TopBuilder, Error> {
        Ok(TopBuilder::default())
    }
    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<TopBuilder, Error> {
        Ok(TopBuilder::default())
    }
    fn serialize_some<T: ?Sized + Serialize>(self, v: &T) -> Result<Self::Ok, Error> {
        v.serialize(self)
    }
    fn serialize_none(self) -> Result<Self::Ok, Error> {
        Ok(ParamsMap::new())
    }
    fn serialize_unit(self) -> Result<Self::Ok, Error> {
        Ok(ParamsMap::new())
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Error> {
        Ok(ParamsMap::new())
    }

    fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Error> {
        Error::unsupported("a non-struct at the query top level")
    }
    fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Error> {
        Error::unsupported("a number at the top level")
    }
    fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Error> {
        Error::unsupported("a number at the top level")
    }
    fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Error> {
        Error::unsupported("a number at the top level")
    }
    fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Error> {
        Error::unsupported("a number at the top level")
    }
    fn serialize_i128(self, _v: i128) -> Result<Self::Ok, Error> {
        Error::unsupported("a number at the top level")
    }
    fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Error> {
        Error::unsupported("a number at the top level")
    }
    fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Error> {
        Error::unsupported("a number at the top level")
    }
    fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Error> {
        Error::unsupported("a number at the top level")
    }
    fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Error> {
        Error::unsupported("a number at the top level")
    }
    fn serialize_u128(self, _v: u128) -> Result<Self::Ok, Error> {
        Error::unsupported("a number at the top level")
    }
    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Error> {
        Error::unsupported("a number at the top level")
    }
    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Error> {
        Error::unsupported("a number at the top level")
    }
    fn serialize_char(self, _v: char) -> Result<Self::Ok, Error> {
        Error::unsupported("a char at the top level")
    }
    fn serialize_str(self, _v: &str) -> Result<Self::Ok, Error> {
        Error::unsupported("a string at the top level")
    }
    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Error> {
        Error::unsupported("bytes at the top level")
    }
    fn serialize_unit_variant(
        self,
        _n: &'static str,
        _i: u32,
        _v: &'static str,
    ) -> Result<Self::Ok, Error> {
        Error::unsupported("a unit variant at the top level")
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _n: &'static str,
        v: &T,
    ) -> Result<Self::Ok, Error> {
        v.serialize(self)
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _n: &'static str,
        _i: u32,
        _v: &'static str,
        _val: &T,
    ) -> Result<Self::Ok, Error> {
        Error::unsupported("a newtype variant at the top level")
    }
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Error> {
        Error::unsupported("a sequence at the top level")
    }
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Error> {
        Error::unsupported("a tuple at the top level")
    }
    fn serialize_tuple_struct(
        self,
        _n: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Error> {
        Error::unsupported("a tuple struct at the top level")
    }
    fn serialize_tuple_variant(
        self,
        _n: &'static str,
        _i: u32,
        _v: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Error> {
        Error::unsupported("a tuple variant at the top level")
    }
    fn serialize_struct_variant(
        self,
        _n: &'static str,
        _i: u32,
        _v: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Error> {
        Error::unsupported("a struct variant at the top level")
    }
}

#[derive(Default)]
struct TopBuilder {
    out: ParamsMap,
    key: Option<String>,
}

impl TopBuilder {
    fn push(&mut self, key: &str, value: Field) {
        match value {
            Field::Scalar(s) => {
                self.out.insert(key.to_string(), s);
            }
            Field::Pairs(subs) => {
                for (sub, v) in subs {
                    self.out.insert(format!("{key}.{sub}"), v);
                }
            }
            Field::Skip => {}
        }
    }
}

impl SerializeStruct for TopBuilder {
    type Ok = ParamsMap;
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        let field = value.serialize(ValueSerializer)?;
        self.push(key, field);
        Ok(())
    }
    fn end(self) -> Result<Self::Ok, Error> {
        Ok(self.out)
    }
}

impl SerializeMap for TopBuilder {
    type Ok = ParamsMap;
    type Error = Error;
    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), Error> {
        self.key = Some(key.serialize(StringSerializer)?);
        Ok(())
    }
    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Error> {
        let key = self
            .key
            .take()
            .ok_or_else(|| Error::custom("value before key"))?;
        let field = value.serialize(ValueSerializer)?;
        self.push(&key, field);
        Ok(())
    }
    fn end(self) -> Result<Self::Ok, Error> {
        Ok(self.out)
    }
}

enum Field {
    Scalar(String),
    Pairs(Vec<(String, String)>),
    Skip,
}

struct ValueSerializer;

impl Serializer for ValueSerializer {
    type Ok = Field;
    type Error = Error;
    type SerializeMap = SubBuilder;
    type SerializeStruct = SubBuilder;
    type SerializeSeq = Impossible<Self::Ok, Error>;
    type SerializeTuple = Impossible<Self::Ok, Error>;
    type SerializeTupleStruct = Impossible<Self::Ok, Error>;
    type SerializeTupleVariant = Impossible<Self::Ok, Error>;
    type SerializeStructVariant = Impossible<Self::Ok, Error>;

    fn serialize_none(self) -> Result<Field, Error> {
        Ok(Field::Skip)
    }
    fn serialize_unit(self) -> Result<Field, Error> {
        Ok(Field::Skip)
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Field, Error> {
        Ok(Field::Skip)
    }
    fn serialize_some<T: ?Sized + Serialize>(self, v: &T) -> Result<Field, Error> {
        v.serialize(self)
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<SubBuilder, Error> {
        Ok(SubBuilder::default())
    }
    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<SubBuilder, Error> {
        Ok(SubBuilder::default())
    }

    fn serialize_bool(self, v: bool) -> Result<Field, Error> {
        Ok(Field::Scalar(v.to_string()))
    }
    fn serialize_i8(self, v: i8) -> Result<Field, Error> {
        Ok(Field::Scalar(v.to_string()))
    }
    fn serialize_i16(self, v: i16) -> Result<Field, Error> {
        Ok(Field::Scalar(v.to_string()))
    }
    fn serialize_i32(self, v: i32) -> Result<Field, Error> {
        Ok(Field::Scalar(v.to_string()))
    }
    fn serialize_i64(self, v: i64) -> Result<Field, Error> {
        Ok(Field::Scalar(v.to_string()))
    }
    fn serialize_i128(self, v: i128) -> Result<Field, Error> {
        Ok(Field::Scalar(v.to_string()))
    }
    fn serialize_u8(self, v: u8) -> Result<Field, Error> {
        Ok(Field::Scalar(v.to_string()))
    }
    fn serialize_u16(self, v: u16) -> Result<Field, Error> {
        Ok(Field::Scalar(v.to_string()))
    }
    fn serialize_u32(self, v: u32) -> Result<Field, Error> {
        Ok(Field::Scalar(v.to_string()))
    }
    fn serialize_u64(self, v: u64) -> Result<Field, Error> {
        Ok(Field::Scalar(v.to_string()))
    }
    fn serialize_u128(self, v: u128) -> Result<Field, Error> {
        Ok(Field::Scalar(v.to_string()))
    }
    fn serialize_f32(self, v: f32) -> Result<Field, Error> {
        Ok(Field::Scalar(v.to_string()))
    }
    fn serialize_f64(self, v: f64) -> Result<Field, Error> {
        Ok(Field::Scalar(v.to_string()))
    }
    fn serialize_char(self, v: char) -> Result<Field, Error> {
        Ok(Field::Scalar(v.to_string()))
    }
    fn serialize_str(self, v: &str) -> Result<Field, Error> {
        Ok(Field::Scalar(v.to_string()))
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _idx: u32,
        variant: &'static str,
    ) -> Result<Field, Error> {
        Ok(Field::Scalar(variant.to_string()))
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        v: &T,
    ) -> Result<Field, Error> {
        v.serialize(self)
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<Field, Error> {
        Error::unsupported("bytes")
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _idx: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Field, Error> {
        Error::unsupported("an enum newtype variant")
    }
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Error> {
        Error::unsupported("a sequence")
    }
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Error> {
        Error::unsupported("a tuple")
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Error> {
        Error::unsupported("a tuple struct")
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _idx: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Error> {
        Error::unsupported("a tuple variant")
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _idx: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Error> {
        Error::unsupported("a struct variant")
    }
}

#[derive(Default)]
struct SubBuilder {
    out: Vec<(String, String)>,
    key: Option<String>,
}

impl SerializeMap for SubBuilder {
    type Ok = Field;
    type Error = Error;
    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), Error> {
        self.key = Some(key.serialize(StringSerializer)?);
        Ok(())
    }
    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Error> {
        let key = self
            .key
            .take()
            .ok_or_else(|| Error::custom("value before key"))?;
        let value = value.serialize(StringSerializer)?;
        self.out.push((key, value));
        Ok(())
    }
    fn end(self) -> Result<Field, Error> {
        Ok(Field::Pairs(self.out))
    }
}

impl SerializeStruct for SubBuilder {
    type Ok = Field;
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        let value = value.serialize(StringSerializer)?;
        self.out.push((key.to_string(), value));
        Ok(())
    }
    fn end(self) -> Result<Field, Error> {
        Ok(Field::Pairs(self.out))
    }
}

struct StringSerializer;

impl Serializer for StringSerializer {
    type Ok = String;
    type Error = Error;
    type SerializeMap = Impossible<String, Error>;
    type SerializeStruct = Impossible<String, Error>;
    type SerializeSeq = Impossible<String, Error>;
    type SerializeTuple = Impossible<String, Error>;
    type SerializeTupleStruct = Impossible<String, Error>;
    type SerializeTupleVariant = Impossible<String, Error>;
    type SerializeStructVariant = Impossible<String, Error>;

    fn serialize_some<T: ?Sized + Serialize>(self, v: &T) -> Result<String, Error> {
        v.serialize(self)
    }
    fn serialize_none(self) -> Result<String, Error> {
        Error::unsupported("an absent value as a key/leaf")
    }
    fn serialize_unit(self) -> Result<String, Error> {
        Error::unsupported("a unit as a key/leaf")
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<String, Error> {
        Error::unsupported("a unit struct as a key/leaf")
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Error> {
        Error::unsupported("a map as a key/leaf")
    }
    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Error> {
        Error::unsupported("a struct as a key/leaf")
    }

    fn serialize_bool(self, v: bool) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_i8(self, v: i8) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_i16(self, v: i16) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_i32(self, v: i32) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_i64(self, v: i64) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_i128(self, v: i128) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_u8(self, v: u8) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_u16(self, v: u16) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_u32(self, v: u32) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_u64(self, v: u64) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_u128(self, v: u128) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_f32(self, v: f32) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_f64(self, v: f64) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_char(self, v: char) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_str(self, v: &str) -> Result<String, Error> {
        Ok(v.to_string())
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _idx: u32,
        variant: &'static str,
    ) -> Result<String, Error> {
        Ok(variant.to_string())
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        v: &T,
    ) -> Result<String, Error> {
        v.serialize(self)
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<String, Error> {
        Error::unsupported("bytes as a key/leaf")
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _idx: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<String, Error> {
        Error::unsupported("an enum newtype variant as a key/leaf")
    }
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Error> {
        Error::unsupported("a sequence as a key/leaf")
    }
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Error> {
        Error::unsupported("a tuple as a key/leaf")
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Error> {
        Error::unsupported("a tuple struct as a key/leaf")
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _idx: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Error> {
        Error::unsupported("a tuple variant as a key/leaf")
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _idx: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Error> {
        Error::unsupported("a struct variant as a key/leaf")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[derive(Serialize)]
    struct Sample {
        filters: BTreeMap<String, String>,
        zoom: Option<String>,
        #[serde(flatten)]
        rest: BTreeMap<String, String>,
    }

    #[test]
    fn serializes_fields_into_dotted_paths() {
        let s = Sample {
            filters: BTreeMap::from([("platform".to_string(), "gcp".to_string())]),
            zoom: Some("region".to_string()),
            rest: BTreeMap::from([("utm".to_string(), "tw".to_string())]),
        };
        let out = to_1nest_params(&s).unwrap();
        assert_eq!(out.get("filters.platform").as_deref(), Some("gcp"));
        assert_eq!(out.get("zoom").as_deref(), Some("region"));
        assert_eq!(out.get("utm").as_deref(), Some("tw"));
    }
}
