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
/// Several of these letters are ambiguous. CST (US Central -6:00 vs.
/// China Standard +8:00), AST (Atlantic -4:00 vs. Arabia +3:00), and BST
/// (British Summer Time +1:00 vs. Bangladesh +6:00) all collide. We pick
/// the US/European reading in each case since that's the reading the rest
/// of this table already leans toward.
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
    ("WET", 0),
    ("WEST", 1 * 60),
    ("EET", 2 * 60),
    ("EEST", 3 * 60),
    ("MSK", 3 * 60),
    ("BST", 1 * 60),
    ("AST", -4 * 60),
    ("ADT", -3 * 60),
    ("HST", -10 * 60),
    ("AKST", -9 * 60),
    ("AKDT", -8 * 60),
    ("SGT", 8 * 60),
    ("HKT", 8 * 60),
    ("KST", 9 * 60),
    ("AEST", 10 * 60),
    ("AEDT", 11 * 60),
    ("ACST", 9 * 60 + 30),
    ("ACDT", 10 * 60 + 30),
    ("AWST", 8 * 60),
    ("NZST", 12 * 60),
    ("NZDT", 13 * 60),
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
        ("WET", "+00:00"),
        ("WEST", "+01:00"),
        ("EET", "+02:00"),
        ("EEST", "+03:00"),
        ("MSK", "+03:00"),
        ("BST", "+01:00"), // British Summer Time, not Bangladesh
        ("AST", "-04:00"), // Atlantic Standard Time, not Arabia
        ("ADT", "-03:00"),
        ("HST", "-10:00"),
        ("AKST", "-09:00"),
        ("AKDT", "-08:00"),
        ("SGT", "+08:00"),
        ("HKT", "+08:00"),
        ("KST", "+09:00"),
        ("AEST", "+10:00"),
        ("AEDT", "+11:00"),
        ("ACST", "+09:30"),
        ("ACDT", "+10:30"),
        ("AWST", "+08:00"),
        ("NZST", "+12:00"),
        ("NZDT", "+13:00"),
        ("nzdt", "+13:00"), // case-insensitivity on a newer entry
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

    /// Tiny deterministic xorshift64 PRNG so the fuzz-style tests below
    /// don't need an external crate, and still reproduce the exact same
    /// sequence every run so a failure is reproducible.
    struct Rng(u64);

    impl Rng {
        fn new(seed: u64) -> Self {
            Rng(seed)
        }

        fn next_u64(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            x
        }

        /// Random integer in `0..bound`.
        fn below(&mut self, bound: u64) -> u64 {
            self.next_u64() % bound
        }

        fn digit_string(&mut self, len: u64) -> String {
            (0..len)
                .map(|_| (b'0' + self.below(10) as u8) as char)
                .collect()
        }
    }

    // parse_numeric's colon branch splits on the first ':' and runs each
    // side through parse_digits independently of width, so the reference
    // check here only needs the documented rules (minutes < 60, hours <=
    // 14, then overall range) rather than reimplementing the parser.
    #[test]
    fn fuzzes_colon_numeric_form_against_reference_rules() {
        let mut rng = Rng::new(0x5EED_1234_ABCD_EF01);

        for _ in 0..5000 {
            let sign_is_negative = rng.below(2) == 0;
            let sign_char = if sign_is_negative { '-' } else { '+' };

            // 0 covers "nothing on this side of the colon"; 1..=3 cover
            // ordinary and oversized digit runs.
            let h_digits = rng.digit_string(rng.below(4));
            let m_digits = rng.digit_string(rng.below(4));

            let text = format!("{sign_char}{h_digits}:{m_digits}");
            let result = parse_numeric(&text);

            if h_digits.is_empty() || m_digits.is_empty() {
                assert_eq!(
                    result,
                    Err(OffsetError::UnrecognizedFormat),
                    "expected empty side of colon to fail for {:?}",
                    text
                );
                continue;
            }

            let hours: i32 = h_digits.parse().unwrap();
            let minutes: i32 = m_digits.parse().unwrap();

            if minutes >= 60 {
                assert_eq!(result, Err(OffsetError::InvalidMinutes), "input {:?}", text);
            } else if hours > 14 {
                assert_eq!(result, Err(OffsetError::InvalidHours), "input {:?}", text);
            } else {
                let sign: i32 = if sign_is_negative { -1 } else { 1 };
                let total = sign * (hours * 60 + minutes);
                if !(MIN_TOTAL_MINUTES as i32..=MAX_TOTAL_MINUTES as i32).contains(&total) {
                    assert_eq!(result, Err(OffsetError::OutOfRange), "input {:?}", text);
                } else {
                    let offset = result.unwrap_or_else(|e| {
                        panic!("input {:?} should have parsed, got error: {}", text, e)
                    });
                    assert_eq!(offset.total_minutes() as i32, total, "input {:?}", text);
                }
            }
        }
    }

    // The no-colon branch picks its hour/minute split from the digit run's
    // length alone (1|2 -> hours only, 3 -> 1+2, 4 -> 2+2, anything else is
    // rejected), so the reference mirrors that shape rule directly.
    #[test]
    fn fuzzes_bare_numeric_widths_against_reference_shape_rules() {
        let mut rng = Rng::new(0xFEED_BEEF_0102_0304);

        for _ in 0..5000 {
            let sign_is_negative = rng.below(2) == 0;
            let sign_char = if sign_is_negative { '-' } else { '+' };
            let digits = rng.digit_string(rng.below(6)); // 0..=5; only 1..=4 are ever valid
            let text = format!("{sign_char}{digits}");

            let result = parse_numeric(&text);

            let (hours, minutes): (i32, i32) = match digits.len() {
                1 | 2 => (digits.parse().unwrap(), 0),
                3 => (digits[..1].parse().unwrap(), digits[1..].parse().unwrap()),
                4 => (digits[..2].parse().unwrap(), digits[2..].parse().unwrap()),
                _ => {
                    assert_eq!(
                        result,
                        Err(OffsetError::UnrecognizedFormat),
                        "input {:?} has an unsupported digit width",
                        text
                    );
                    continue;
                }
            };

            if minutes >= 60 {
                assert_eq!(result, Err(OffsetError::InvalidMinutes), "input {:?}", text);
            } else if hours > 14 {
                assert_eq!(result, Err(OffsetError::InvalidHours), "input {:?}", text);
            } else {
                let sign: i32 = if sign_is_negative { -1 } else { 1 };
                let total = sign * (hours * 60 + minutes);
                if !(MIN_TOTAL_MINUTES as i32..=MAX_TOTAL_MINUTES as i32).contains(&total) {
                    assert_eq!(result, Err(OffsetError::OutOfRange), "input {:?}", text);
                } else {
                    let offset = result.unwrap_or_else(|e| {
                        panic!("input {:?} should have parsed, got error: {}", text, e)
                    });
                    assert_eq!(offset.total_minutes() as i32, total, "input {:?}", text);
                }
            }
        }
    }

    // Pure garbage assembled from a biased alphabet (digits, sign, colon,
    // and the letters that spell UTC/GMT/PST-style tokens). The only
    // property worth asserting on nonsense is that it never panics; when it
    // happens to parse, its own canonical form must reparse to itself.
    #[test]
    fn fuzz_normalize_never_panics_on_arbitrary_input() {
        let mut rng = Rng::new(0xABCD_EF01_2345_6789);
        let alphabet: &[char] = &[
            '+', '-', ':', '.', ' ', '0', '1', '5', '9', 'u', 'U', 't', 'T', 'c', 'C', 'z', 'Z',
            'p', 'P', 's', 'S',
        ];

        for _ in 0..5000 {
            let len = rng.below(12);
            let text: String = (0..len)
                .map(|_| alphabet[rng.below(alphabet.len() as u64) as usize])
                .collect();

            if let Ok(offset) = normalize(&text) {
                let formatted = offset.to_string();
                assert_eq!(
                    normalize(&formatted),
                    Ok(offset),
                    "canonical form of {:?} ({}) didn't reparse to itself",
                    text,
                    formatted
                );
            }
        }
    }
}
