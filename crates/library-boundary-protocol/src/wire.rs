use std::fmt;

use serde::de::Visitor;

pub(crate) mod i64_string {
    use super::*;

    pub(crate) fn serialize<S>(value: &i64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<i64, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(I64StringVisitor)
    }
}

pub(crate) mod option_i64_string {
    use super::*;

    pub(crate) fn serialize<S>(value: &Option<i64>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match value {
            Some(value) => serializer.serialize_some(&value.to_string()),
            None => serializer.serialize_none(),
        }
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_option(OptionI64StringVisitor)
    }
}

pub(crate) fn parse_u64_string<E>(value: &str) -> Result<u64, E>
where
    E: serde::de::Error,
{
    value.parse::<u64>().map_err(E::custom)
}

pub(crate) fn parse_i64_string<E>(value: &str) -> Result<i64, E>
where
    E: serde::de::Error,
{
    value.parse::<i64>().map_err(E::custom)
}

struct I64StringVisitor;

impl Visitor<'_> for I64StringVisitor {
    type Value = i64;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an i64 encoded as a JSON string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        parse_i64_string(value)
    }
}

struct OptionI64StringVisitor;

impl<'de> Visitor<'de> for OptionI64StringVisitor {
    type Value = Option<i64>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("null or an i64 encoded as a JSON string")
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(None)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(I64StringVisitor).map(Some)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(None)
    }
}
