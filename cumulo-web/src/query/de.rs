use leptos_router::params::ParamsMap;
use serde::de::value::MapDeserializer;
use serde::de::{Deserialize, Deserializer, IntoDeserializer, Visitor};

use super::error::Error;

pub fn from_1nest_params<'de, T: Deserialize<'de>>(params: &ParamsMap) -> Result<T, Error> {
    T::deserialize(MapDeserializer::new(Value::group(params).into_iter()))
}

enum Value {
    Scalar(String),
    Map(Vec<(String, String)>),
}

impl Value {
    fn group(params: &ParamsMap) -> Vec<(String, Value)> {
        let mut out: Vec<(String, Value)> = Vec::new();
        for (key, value) in params {
            match key.split_once('.') {
                Some((head, sub)) => match out.iter_mut().find(|(k, _)| k == head) {
                    Some((_, Value::Map(entries))) => {
                        entries.push((sub.to_string(), value.to_string()))
                    }
                    _ => out.push((
                        head.to_string(),
                        Value::Map(vec![(sub.to_string(), value.to_string())]),
                    )),
                },
                None => out.push((key.to_string(), Value::Scalar(value.to_string()))),
            }
        }
        out
    }
}

impl<'de> IntoDeserializer<'de, Error> for Value {
    type Deserializer = ValueDeserializer;
    fn into_deserializer(self) -> ValueDeserializer {
        ValueDeserializer(self)
    }
}

struct ValueDeserializer(Value);

impl<'de> Deserializer<'de> for ValueDeserializer {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.0 {
            Value::Scalar(s) => visitor.visit_string(s),
            Value::Map(entries) => {
                MapDeserializer::new(entries.into_iter()).deserialize_any(visitor)
            }
        }
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_some(self)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        match self.0 {
            Value::Scalar(s) => s
                .into_deserializer()
                .deserialize_enum(name, variants, visitor),
            other => ValueDeserializer(other).deserialize_any(visitor),
        }
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct seq tuple tuple_struct map struct
        identifier ignored_any
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::collections::BTreeMap;

    #[derive(Debug, Default, PartialEq, Deserialize)]
    struct Sample {
        #[serde(default)]
        filters: BTreeMap<String, String>,
        #[serde(default)]
        zoom: Option<String>,
        #[serde(flatten)]
        rest: BTreeMap<String, String>,
    }

    fn params(p: &[(&str, &str)]) -> ParamsMap {
        let mut m = ParamsMap::new();
        for (k, v) in p {
            m.insert(k.to_string(), v.to_string());
        }
        m
    }

    #[test]
    fn deserializes_scalar_into_enum_variant() {
        #[derive(Debug, PartialEq, Deserialize)]
        #[serde(rename_all = "lowercase")]
        enum Mode {
            Alpha,
            Beta,
        }
        #[derive(Debug, PartialEq, Deserialize)]
        struct S {
            mode: Mode,
        }
        let s: S = from_1nest_params(&params(&[("mode", "beta")])).unwrap();
        assert_eq!(s.mode, Mode::Beta);
    }

    #[test]
    fn deserializes_dotted_paths_into_fields() {
        let s: Sample = from_1nest_params(&params(&[
            ("filters.platform", "gcp"),
            ("filters.env", "prod"),
            ("zoom", "region"),
            ("utm", "tw"),
        ]))
        .unwrap();
        assert_eq!(s.filters.get("platform").map(String::as_str), Some("gcp"));
        assert_eq!(s.filters.get("env").map(String::as_str), Some("prod"));
        assert_eq!(s.zoom.as_deref(), Some("region"));
        assert_eq!(s.rest.get("utm").map(String::as_str), Some("tw"));
    }
}
