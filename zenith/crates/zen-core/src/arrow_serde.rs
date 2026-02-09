//! Serde helpers for Arrow-compatible serialization of chrono types.
//!
//! These modules serialize Rust chrono types to Arrow-native numeric formats
//! instead of strings. Use with `#[serde(with = "arrow_serde::...")]` on struct fields.
//!
//! Ported from aether's `aether-types::utils::arrow_serde`.
//!
//! # Example
//! ```ignore
//! use zen_core::arrow_serde;
//! use chrono::{DateTime, NaiveDate, Utc};
//!
//! #[derive(Serialize, Deserialize)]
//! struct Record {
//!     #[serde(with = "arrow_serde::date32")]
//!     pub date: NaiveDate,
//!
//!     #[serde(with = "arrow_serde::timestamp_micros_utc")]
//!     pub created_at: DateTime<Utc>,
//!
//!     #[serde(with = "arrow_serde::timestamp_micros_utc_option")]
//!     pub updated_at: Option<DateTime<Utc>>,
//! }
//! ```

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

const UNIX_EPOCH_DATE: NaiveDate = match NaiveDate::from_ymd_opt(1970, 1, 1) {
    Some(d) => d,
    None => panic!("Invalid epoch date"),
};

/// Serialize `NaiveDate` as i32 days since Unix epoch (1970-01-01).
/// Compatible with Arrow `Date32` type.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::wildcard_imports
)]
pub mod date32 {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub fn serialize<S: Serializer>(date: &NaiveDate, s: S) -> Result<S::Ok, S::Error> {
        let days = date.signed_duration_since(UNIX_EPOCH_DATE).num_days() as i32;
        days.serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<NaiveDate, D::Error> {
        let days = i32::deserialize(d)?;
        if days >= 0 {
            UNIX_EPOCH_DATE
                .checked_add_days(chrono::Days::new(days as u64))
                .ok_or_else(|| serde::de::Error::custom("Days overflow"))
        } else {
            UNIX_EPOCH_DATE
                .checked_sub_days(chrono::Days::new((-days) as u64))
                .ok_or_else(|| serde::de::Error::custom("Days underflow"))
        }
    }
}

/// Serialize `Option<NaiveDate>` as `Option<i32>` days since epoch.
/// Compatible with nullable Arrow `Date32` type.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::wildcard_imports
)]
pub mod date32_option {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub fn serialize<S: Serializer>(date: &Option<NaiveDate>, s: S) -> Result<S::Ok, S::Error> {
        match date {
            Some(d) => {
                let days = d.signed_duration_since(UNIX_EPOCH_DATE).num_days() as i32;
                s.serialize_some(&days)
            }
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<NaiveDate>, D::Error> {
        let opt = Option::<i32>::deserialize(d)?;
        match opt {
            Some(days) => {
                let date = if days >= 0 {
                    UNIX_EPOCH_DATE
                        .checked_add_days(chrono::Days::new(days as u64))
                        .ok_or_else(|| serde::de::Error::custom("Days overflow"))?
                } else {
                    UNIX_EPOCH_DATE
                        .checked_sub_days(chrono::Days::new((-days) as u64))
                        .ok_or_else(|| serde::de::Error::custom("Days underflow"))?
                };
                Ok(Some(date))
            }
            None => Ok(None),
        }
    }
}

/// Serialize `DateTime<Utc>` as i64 microseconds since Unix epoch.
/// Compatible with Arrow `Timestamp(Microsecond, Some("UTC"))` type.
#[allow(clippy::wildcard_imports)]
pub mod timestamp_micros_utc {
    use super::*;

    pub fn serialize<S: Serializer>(dt: &DateTime<Utc>, s: S) -> Result<S::Ok, S::Error> {
        dt.timestamp_micros().serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<DateTime<Utc>, D::Error> {
        let micros = i64::deserialize(d)?;
        Utc.timestamp_micros(micros)
            .single()
            .ok_or_else(|| serde::de::Error::custom("Invalid timestamp microseconds"))
    }
}

/// Serialize `Option<DateTime<Utc>>` as `Option<i64>` microseconds.
/// Compatible with nullable Arrow `Timestamp(Microsecond, Some("UTC"))` type.
#[allow(clippy::wildcard_imports)]
pub mod timestamp_micros_utc_option {
    use super::*;

    pub fn serialize<S: Serializer>(dt: &Option<DateTime<Utc>>, s: S) -> Result<S::Ok, S::Error> {
        match dt {
            Some(d) => s.serialize_some(&d.timestamp_micros()),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<DateTime<Utc>>, D::Error> {
        let opt = Option::<i64>::deserialize(d)?;
        match opt {
            Some(micros) => {
                let dt = Utc
                    .timestamp_micros(micros)
                    .single()
                    .ok_or_else(|| serde::de::Error::custom("Invalid timestamp microseconds"))?;
                Ok(Some(dt))
            }
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    struct DateTest {
        #[serde(with = "super::date32")]
        date: NaiveDate,
    }

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    struct TimestampTest {
        #[serde(with = "super::timestamp_micros_utc")]
        ts: DateTime<Utc>,
    }

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    struct TimestampOptionTest {
        #[serde(with = "super::timestamp_micros_utc_option")]
        ts: Option<DateTime<Utc>>,
    }

    #[test]
    fn test_date32_roundtrip() {
        let test = DateTest {
            date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
        };
        let json = serde_json::to_string(&test).unwrap();
        assert_eq!(json, r#"{"date":19737}"#);
        let recovered: DateTest = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, test);
    }

    #[test]
    fn test_timestamp_micros_roundtrip() {
        let test = TimestampTest {
            ts: Utc.timestamp_micros(1_705_312_200_000_000).unwrap(),
        };
        let json = serde_json::to_string(&test).unwrap();
        assert_eq!(json, r#"{"ts":1705312200000000}"#);
        let recovered: TimestampTest = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, test);
    }

    #[test]
    fn test_timestamp_option_none() {
        let test = TimestampOptionTest { ts: None };
        let json = serde_json::to_string(&test).unwrap();
        assert_eq!(json, r#"{"ts":null}"#);
        let recovered: TimestampOptionTest = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, test);
    }
}
