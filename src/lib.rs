//! Parses the many ways people type a UTC offset ("+5:30", "UTC+05:30",
//! "utc+5", "-0800", "PST", "Z") and formats them all the same way.

use std::fmt;

/// A UTC offset, stored as signed minutes so `-00:00` and `+00:00`
/// collapse to the same value automatically (there's no negative zero
/// in integer arithmetic).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Offset {
    total_minutes: i16,
}

/// Real-world offsets run from -12:00 (Baker Island) to +14:00 (Line
/// Islands). Anything outside that is not a timezone, just a number.
const MIN_TOTAL_MINUTES: i16 = -12 * 60;
const MAX_TOTAL_MINUTES: i16 = 14 * 60;

impl Offset {
    pub fn from_minutes(total_minutes: i16) -> Result<Self, OffsetError> {
        if total_minutes < MIN_TOTAL_MINUTES || total_minutes > MAX_TOTAL_MINUTES {
            return Err(OffsetError::OutOfRange);
        }
        Ok(Offset { total_minutes })
    }

    pub fn total_minutes(&self) -> i16 {
        self.total_minutes
    }

    pub fn hours(&self) -> i16 {
        self.total_minutes.abs() / 60
    }

    pub fn minutes(&self) -> i16 {
        self.total_minutes.abs() % 60
    }

    pub fn is_negative(&self) -> bool {
        self.total_minutes < 0
    }
}

impl fmt::Display for Offset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.is_negative() { '-' } else { '+' };
        write!(f, "{}{:02}:{:02}", sign, self.hours(), self.minutes())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetError {
    Empty,
    OutOfRange,
    InvalidHours,
    InvalidMinutes,
    UnrecognizedFormat,
}

impl fmt::Display for OffsetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            OffsetError::Empty => "input is empty",
            OffsetError::OutOfRange => "offset falls outside -12:00..+14:00",
            OffsetError::InvalidHours => "hour part is too large to be a real offset",
            OffsetError::InvalidMinutes => "minute part must be between 00 and 59",
            OffsetError::UnrecognizedFormat => {
                "not a recognized offset or timezone abbreviation"
            }
        };
        write!(f, "{}", msg)
    }
}

impl std::error::Error for OffsetError {}

/// Fixed-offset abbreviations. This is intentionally a short, hand-picked
/// list rather than the full IANA database: those need a rules engine for
/// DST transitions by date, which is out of scope for a string formatter.
///
/// CST is ambiguous (US Central: -6:00, China Standard: +8:00). We pick
/// the US reading since the rest of this table is US/European military
/// zone letters, not IANA zone names.
const NAMED_ZONES: &[(&str, i16)] = &[
    ("UTC", 0),
    ("GMT", 0),
    ("Z", 0),
    ("EST", -5 * 60),
    ("EDT", -4 * 60),
    ("CST", -6 * 60),
    ("CDT", -5 * 60),
    ("MST", -7 * 60),
    ("MDT", -6 * 60),
    ("PST", -8 * 60),
    ("PDT", -7 * 60),
    ("IST", 5 * 60 + 30),
    ("JST", 9 * 60),
    ("CET", 1 * 60),
    ("CEST", 2 * 60),
];

/// Normalizes messy offset/timezone text into a canonical [`Offset`].
///
/// Accepts, case-insensitively: named abbreviations from a small fixed
/// table ("UTC", "PST", "Z", ...), those same names glued to a numeric
/// offset ("UTC+8", "GMT-05:30"), and bare signed numeric offsets in
/// several widths ("+5", "+05:30", "+0530", "+530").
pub fn normalize(input: &str) -> Result<Offset, OffsetError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(OffsetError::Empty);
    }

    let upper = trimmed.to_ascii_uppercase();

    for prefix in ["UTC", "GMT"] {
        if let Some(rest) = upper.strip_prefix(prefix) {
            if rest.is_empty() {
                return Offset::from_minutes(0);
            }
            if rest.starts_with('+') || rest.starts_with('-') {
                return parse_numeric(rest);
            }
        }
    }

    if let Some(&(_, minutes)) = NAMED_ZONES.iter().find(|(name, _)| *name == upper) {
        return Offset::from_minutes(minutes);
    }

    parse_numeric(&upper)
}

/// Pulls a UTC offset out of a full datetime string and normalizes it,
/// ignoring the date/time portion. This crate does not parse dates, so it
/// only looks at the tail of the string: a `Z`/`z` marker, a named zone
/// token separated by whitespace ("... GMT-0800", "... PST"), or a signed
/// numeric offset glued directly onto a time ("...T10:30:00+05:30").
///
/// If the whole input is already a bare offset, this behaves like
/// [`normalize`].
pub fn extract_offset(input: &str) -> Result<Offset, OffsetError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(OffsetError::Empty);
    }

    if let Ok(offset) = normalize(trimmed) {
        return Ok(offset);
    }

    if let Some(rest) = trimmed.strip_suffix(|c: char| c == 'Z' || c == 'z') {
        if rest.ends_with(|c: char| c.is_ascii_digit()) {
            return Offset::from_minutes(0);
        }
    }

    if let Some(idx) = trimmed.rfind(char::is_whitespace) {
        if let Ok(offset) = normalize(&trimmed[idx + 1..]) {
            return Ok(offset);
        }
    }

    if let Some(idx) = trimmed.rfind(['+', '-']) {
        if let Ok(offset) = parse_numeric(&trimmed[idx..].to_ascii_uppercase()) {
            return Ok(offset);
        }
    }

    Err(OffsetError::UnrecognizedFormat)
}

fn parse_numeric(s: &str) -> Result<Offset, OffsetError> {
    let mut chars = s.chars();
    let sign: i16 = match chars.next() {
        Some('+') => 1,
        Some('-') => -1,
        _ => return Err(OffsetError::UnrecognizedFormat),
    };
    let rest: String = chars.collect();

    let (hours, minutes) = if let Some((h, m)) = rest.split_once(':') {
        (parse_digits(h)?, parse_digits(m)?)
    } else {
        match rest.len() {
            1 | 2 => (parse_digits(&rest)?, 0),
            3 => (parse_digits(&rest[..1])?, parse_digits(&rest[1..])?),
            4 => (parse_digits(&rest[..2])?, parse_digits(&rest[2..])?),
            _ => return Err(OffsetError::UnrecognizedFormat),
        }
    };

    if minutes >= 60 {
        return Err(OffsetError::InvalidMinutes);
    }
    if hours > 14 {
        return Err(OffsetError::InvalidHours);
    }

    Offset::from_minutes(sign * (hours * 60 + minutes))
}

fn parse_digits(s: &str) -> Result<i16, OffsetError> {
    if s.is_empty() || !s.chars().all(|c| c.is_ascii_digit()) {
        return Err(OffsetError::UnrecognizedFormat);
    }
    s.parse().map_err(|_| OffsetError::UnrecognizedFormat)
}

#[cfg(test)]
mod tests {
    use super::*;

    // (input, expected canonical output) for inputs that should succeed.
    const OK_CASES: &[(&str, &str)] = &[
        ("UTC", "+00:00"),
        ("gmt", "+00:00"),
        ("Z", "+00:00"),
        ("z", "+00:00"),
        ("  +05:30  ", "+05:30"),
        ("+0530", "+05:30"),
        ("+530", "+05:30"),
        ("+5", "+05:00"),
        ("-8", "-08:00"),
        ("-08:00", "-08:00"),
        ("UTC+8", "+08:00"),
        ("utc+8", "+08:00"),
        ("UTC-05:00", "-05:00"),
        ("GMT+5:30", "+05:30"),
        ("+5:5", "+05:05"),
        ("-00:00", "+00:00"), // negative zero collapses
        ("+00:00", "+00:00"),
        ("+14:00", "+14:00"), // max boundary
        ("-12:00", "-12:00"), // min boundary
        ("PST", "-08:00"),
        ("PDT", "-07:00"),
        ("IST", "+05:30"),
        ("CEST", "+02:00"),
    ];

    // (input, expected error) for inputs that should be rejected.
    const ERR_CASES: &[(&str, OffsetError)] = &[
        ("", OffsetError::Empty),
        ("   ", OffsetError::Empty),
        ("banana", OffsetError::UnrecognizedFormat),
        ("530", OffsetError::UnrecognizedFormat), // no sign, not a named zone
        ("UTC+", OffsetError::UnrecognizedFormat),
        ("+15:00", OffsetError::InvalidHours),
        ("+05:60", OffsetError::InvalidMinutes),
        ("+14:01", OffsetError::OutOfRange), // valid hours/minutes, out of overall range
        ("-12:01", OffsetError::OutOfRange),
    ];

    #[test]
    fn normalizes_awkward_inputs() {
        for (input, expected) in OK_CASES {
            match normalize(input) {
                Ok(offset) => assert_eq!(
                    offset.to_string(),
                    *expected,
                    "input {:?} normalized wrong",
                    input
                ),
                Err(e) => panic!("input {:?} should have parsed, got error: {}", input, e),
            }
        }
    }

    #[test]
    fn rejects_broken_inputs() {
        for (input, expected_err) in ERR_CASES {
            match normalize(input) {
                Ok(offset) => panic!(
                    "input {:?} should have failed, got {}",
                    input, offset
                ),
                Err(e) => assert_eq!(e, *expected_err, "input {:?} gave wrong error", input),
            }
        }
    }

    // A colon splits the input into two digit runs, and either side can be
    // missing or non-numeric. These used to only be covered incidentally;
    // spell them out so a future change to the split_once path can't
    // silently start accepting garbage.
    const MALFORMED_COLON_CASES: &[(&str, OffsetError)] = &[
        ("+5:", OffsetError::UnrecognizedFormat), // nothing after the colon
        ("+:30", OffsetError::UnrecognizedFormat), // nothing before the colon
        ("+:", OffsetError::UnrecognizedFormat), // both sides empty
        ("+05:30:00", OffsetError::UnrecognizedFormat), // second colon lands in the minute run
        ("+05:3a", OffsetError::UnrecognizedFormat), // non-digit minute
        ("+0a:30", OffsetError::UnrecognizedFormat), // non-digit hour
        ("-5:", OffsetError::UnrecognizedFormat), // same shape, negative sign
    ];

    #[test]
    fn rejects_malformed_colon_input() {
        for (input, expected_err) in MALFORMED_COLON_CASES {
            match normalize(input) {
                Ok(offset) => panic!(
                    "input {:?} should have failed, got {}",
                    input, offset
                ),
                Err(e) => assert_eq!(e, *expected_err, "input {:?} gave wrong error", input),
            }
        }
    }

    const EXTRACT_OK_CASES: &[(&str, &str)] = &[
        ("2024-01-15T10:30:00+05:30", "+05:30"),
        ("2024-01-15T10:30:00-08:00", "-08:00"),
        ("2024-01-15T10:30:00Z", "+00:00"),
        ("2024-01-15T10:30:00z", "+00:00"),
        ("2024-01-15T10:30:00-0800", "-08:00"),
        ("Jan 15 2024 10:30:00 GMT-0800", "-08:00"),
        ("Jan 15 2024 10:30:00 UTC", "+00:00"),
        ("2024-01-15 10:30:00 PST", "-08:00"),
        ("+05:30", "+05:30"), // bare offset, no datetime around it
        ("UTC+8", "+08:00"),
    ];

    const EXTRACT_ERR_CASES: &[(&str, OffsetError)] = &[
        ("", OffsetError::Empty),
        ("   ", OffsetError::Empty),
        ("2024-01-15T10:30:00", OffsetError::UnrecognizedFormat), // no offset at all
        ("2024-01-15 10:30:00", OffsetError::UnrecognizedFormat),
        ("just some text", OffsetError::UnrecognizedFormat),
    ];

    #[test]
    fn extracts_offset_from_datetime_strings() {
        for (input, expected) in EXTRACT_OK_CASES {
            match extract_offset(input) {
                Ok(offset) => assert_eq!(
                    offset.to_string(),
                    *expected,
                    "input {:?} extracted wrong",
                    input
                ),
                Err(e) => panic!("input {:?} should have parsed, got error: {}", input, e),
            }
        }
    }

    #[test]
    fn rejects_datetime_strings_without_an_offset() {
        for (input, expected_err) in EXTRACT_ERR_CASES {
            match extract_offset(input) {
                Ok(offset) => panic!(
                    "input {:?} should have failed, got {}",
                    input, offset
                ),
                Err(e) => assert_eq!(e, *expected_err, "input {:?} gave wrong error", input),
            }
        }
    }
}
