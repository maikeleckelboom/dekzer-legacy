use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CapabilityKind(String);

impl CapabilityKind {
    pub const WAVEFORM: &'static str = "waveform";
    pub const TEMPO: &'static str = "tempo";
    pub const MUSICAL_KEY: &'static str = "musical_key";
    pub const BEATGRID: &'static str = "beatgrid";
    pub const STEMS: &'static str = "stems";

    pub fn new(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        if is_valid_capability_kind(&value) {
            Some(Self(value))
        } else {
            None
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::new(value.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }

    pub fn waveform() -> Self {
        Self::known(Self::WAVEFORM)
    }

    pub fn tempo() -> Self {
        Self::known(Self::TEMPO)
    }

    pub fn musical_key() -> Self {
        Self::known(Self::MUSICAL_KEY)
    }

    pub fn beatgrid() -> Self {
        Self::known(Self::BEATGRID)
    }

    pub fn stems() -> Self {
        Self::known(Self::STEMS)
    }

    fn known(value: &'static str) -> Self {
        debug_assert!(is_valid_capability_kind(value));
        Self(value.to_string())
    }
}

impl fmt::Display for CapabilityKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for CapabilityKind {
    type Err = CapabilityKindParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value).ok_or(CapabilityKindParseError)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityKindParseError;

impl fmt::Display for CapabilityKindParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid capability kind")
    }
}

impl std::error::Error for CapabilityKindParseError {}

fn is_valid_capability_kind(value: &str) -> bool {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) if first.is_ascii_lowercase() => {}
        _ => return false,
    }

    chars.all(|character| {
        character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || character == '_'
            || character == '-'
            || character == '.'
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityState {
    Missing,
    Queued,
    Leased,
    Ready,
    Stale,
    Blocked,
    Failed,
}

impl CapabilityState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Queued => "queued",
            Self::Leased => "leased",
            Self::Ready => "ready",
            Self::Stale => "stale",
            Self::Blocked => "blocked",
            Self::Failed => "failed",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "missing" => Some(Self::Missing),
            "queued" => Some(Self::Queued),
            "leased" => Some(Self::Leased),
            "ready" => Some(Self::Ready),
            "stale" => Some(Self::Stale),
            "blocked" => Some(Self::Blocked),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityStabilityClass {
    Provisional,
    Stable,
}

impl CapabilityStabilityClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Provisional => "provisional",
            Self::Stable => "stable",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "provisional" => Some(Self::Provisional),
            "stable" => Some(Self::Stable),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityInvalidationMode {
    MarkStale,
}

impl CapabilityInvalidationMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MarkStale => "mark_stale",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "mark_stale" => Some(Self::MarkStale),
            _ => None,
        }
    }
}
