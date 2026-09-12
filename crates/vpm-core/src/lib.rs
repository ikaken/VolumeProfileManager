use chrono::{Datelike, NaiveDate, NaiveDateTime, Timelike, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct VolumeProfile {
    pub device_id: String,
    pub device_name: String,
    #[serde(
        serialize_with = "serialize_volume",
        deserialize_with = "deserialize_volume"
    )]
    pub master_volume: f32,
    pub is_muted: bool,
    pub created_at: ProfileTimestamp,
    pub last_applied: ProfileTimestamp,
}

fn serialize_volume<S: Serializer>(value: &f32, serializer: S) -> Result<S::Ok, S::Error> {
    if !value.is_finite() {
        return Err(serde::ser::Error::custom("MasterVolume must be finite"));
    }
    serializer.serialize_f32(*value)
}

fn deserialize_volume<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f32, D::Error> {
    let value = f32::deserialize(deserializer)?;
    if !value.is_finite() {
        return Err(serde::de::Error::custom(
            "MasterVolume must be a finite float32",
        ));
    }
    Ok(value)
}

#[derive(Clone, Copy, Debug)]
pub struct ProfileTimestamp {
    value: NaiveDateTime,
    utc: bool,
}

impl ProfileTimestamp {
    pub fn now() -> Self {
        let value = Utc::now().naive_utc();
        Self {
            value: value
                .with_nanosecond(value.nanosecond() / 100 * 100)
                .unwrap(),
            utc: true,
        }
    }
}

impl Default for ProfileTimestamp {
    fn default() -> Self {
        Self {
            value: NaiveDate::from_ymd_opt(1, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            utc: false,
        }
    }
}

impl PartialEq for ProfileTimestamp {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl Eq for ProfileTimestamp {}

impl PartialOrd for ProfileTimestamp {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ProfileTimestamp {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimestampParseError;

impl fmt::Display for TimestampParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("expected a .NET DateTime in years 0001..9999 with at most seven fractional digits and optional Z")
    }
}

impl std::error::Error for TimestampParseError {}

impl FromStr for ProfileTimestamp {
    type Err = TimestampParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let utc = text.ends_with('Z');
        let body = text.strip_suffix('Z').unwrap_or(text);
        if !body.is_ascii() || body.len() < 19 || body.len() > 27 {
            return Err(TimestampParseError);
        }
        for (index, byte) in body.bytes().take(19).enumerate() {
            let valid = match index {
                4 | 7 => byte == b'-',
                10 => byte == b'T',
                13 | 16 => byte == b':',
                _ => byte.is_ascii_digit(),
            };
            if !valid {
                return Err(TimestampParseError);
            }
        }
        if body.len() > 19
            && (body.as_bytes()[19] != b'.'
                || body.len() == 20
                || !body[20..].bytes().all(|b| b.is_ascii_digit()))
        {
            return Err(TimestampParseError);
        }
        let value = NaiveDateTime::parse_from_str(
            body,
            if body.len() > 19 {
                "%Y-%m-%dT%H:%M:%S%.f"
            } else {
                "%Y-%m-%dT%H:%M:%S"
            },
        )
        .map_err(|_| TimestampParseError)?;
        if !(1..=9999).contains(&value.year()) || value.nanosecond() >= 1_000_000_000 {
            return Err(TimestampParseError);
        }
        Ok(Self { value, utc })
    }
}

impl fmt::Display for ProfileTimestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value.format("%Y-%m-%dT%H:%M:%S"))?;
        let ticks = self.value.nanosecond() / 100;
        if ticks != 0 {
            let fraction = format!("{ticks:07}");
            write!(f, ".{}", fraction.trim_end_matches('0'))?;
        }
        if self.utc {
            f.write_str("Z")?;
        }
        Ok(())
    }
}

impl Serialize for ProfileTimestamp {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for ProfileTimestamp {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

pub trait Clock {
    fn now(&self) -> ProfileTimestamp;
    fn elapsed(&self) -> Duration;
}

#[derive(Debug)]
pub struct SystemClock {
    started: Instant,
}

impl Default for SystemClock {
    fn default() -> Self {
        Self {
            started: Instant::now(),
        }
    }
}

impl Clock for SystemClock {
    fn now(&self) -> ProfileTimestamp {
        ProfileTimestamp::now()
    }
    fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }
}

pub trait ProfileStore {
    fn load(&self) -> std::io::Result<Vec<VolumeProfile>>;
    fn save(&self, profile: &VolumeProfile) -> std::io::Result<()>;
    fn delete(&self, identifier: &str) -> std::io::Result<()>;
}

fn ordinal_uppercase(ch: char) -> char {
    match ch {
        '\u{131}'
        | '\u{17f}'
        | '\u{19b}'
        | '\u{264}'
        | '\u{1c8a}'
        | '\u{a7cd}'
        | '\u{a7cf}'
        | '\u{a7d3}'
        | '\u{a7d5}'
        | '\u{a7db}'
        | '\u{16ebb}'..='\u{16ed3}' => ch,
        '\u{1f80}'..='\u{1f87}' | '\u{1f90}'..='\u{1f97}' | '\u{1fa0}'..='\u{1fa7}' => {
            char::from_u32(ch as u32 + 8).unwrap()
        }
        '\u{1fb3}' => '\u{1fbc}',
        '\u{1fc3}' => '\u{1fcc}',
        '\u{1ff3}' => '\u{1ffc}',
        _ => {
            let mut upper = ch.to_uppercase();
            let first = upper.next().unwrap();
            if upper.next().is_none() { first } else { ch }
        }
    }
}

pub fn ordinal_ignore_case_eq(left: &str, right: &str) -> bool {
    left.chars()
        .map(ordinal_uppercase)
        .eq(right.chars().map(ordinal_uppercase))
}

pub fn ordinal_ignore_case_contains(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    let needle: Vec<char> = needle.chars().map(ordinal_uppercase).collect();
    let haystack: Vec<char> = haystack.chars().map(ordinal_uppercase).collect();
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

fn is_blank(text: &str) -> bool {
    text.chars().all(char::is_whitespace)
}

fn normalize_name(name: &str) -> String {
    name.split(' ')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_owned()
}

pub fn match_profile<'a>(
    profiles: &'a [VolumeProfile],
    device_id: &str,
    device_name: Option<&str>,
) -> Option<&'a VolumeProfile> {
    if !is_blank(device_id)
        && let Some(profile) = profiles
            .iter()
            .find(|p| ordinal_ignore_case_eq(&p.device_id, device_id))
    {
        return Some(profile);
    }
    let name = device_name.filter(|name| !is_blank(name))?;
    let target = normalize_name(name);
    let ranked = |partial: bool| {
        let mut best: Option<&VolumeProfile> = None;
        for profile in profiles {
            let normalized = normalize_name(&profile.device_name);
            let matches = if partial {
                !is_blank(&profile.device_name)
                    && (ordinal_ignore_case_contains(&target, &normalized)
                        || ordinal_ignore_case_contains(&normalized, &target))
            } else {
                ordinal_ignore_case_eq(&normalized, &target)
            };
            if matches
                && best.is_none_or(|current| {
                    (profile.last_applied, profile.created_at)
                        > (current.last_applied, current.created_at)
                })
            {
                best = Some(profile);
            }
        }
        best
    };
    ranked(false).or_else(|| ranked(true))
}
