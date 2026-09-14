# tz-offset-fmt

People write UTC offsets in a dozen incompatible ways: `+5:30`, `UTC+05:30`,
`utc+5`, `-0800`, `Z`, `PST`. If you're parsing timestamps from user input,
CSV exports, or old log files, you end up with all of these mixed together.
This crate takes any of them and turns them into one canonical form:
`+HH:MM` (or `-HH:MM`).

It does not do daylight-saving-by-date lookups or full IANA zone names
(`America/Chicago`). It normalizes fixed offsets and a small table of
common abbreviations. If you need "what was the offset in Berlin on this
specific date," you need a real tz database, not this.

## What it accepts

- Named zones (case-insensitive): `UTC`, `GMT`, `Z`, `EST`, `EDT`, `CST`,
  `CDT`, `MST`, `MDT`, `PST`, `PDT`, `IST`, `JST`, `CET`, `CEST`
- Named zone plus a signed offset: `UTC+8`, `GMT-05:30`
- Signed numeric offsets in several widths: `+5`, `+05`, `+530`, `+0530`,
  `+05:30`

Offsets outside `-12:00..+14:00` are rejected, since nothing on Earth uses
them.

## Library usage

```rust
use tz_offset_fmt::normalize;

fn main() {
    for input in ["utc+5:30", "-0800", "Z", "PST"] {
        match normalize(input) {
            Ok(offset) => println!("{input} -> {offset}"),
            Err(err) => println!("{input} -> error: {err}"),
        }
    }
}
```

Output:

```
utc+5:30 -> +05:30
-0800 -> -08:00
Z -> +00:00
PST -> -08:00
```

`normalize` expects the whole input to be an offset. To pull an offset out
of a full timestamp, use `extract_offset` instead, which looks at the tail
of the string:

```rust
use tz_offset_fmt::extract_offset;

fn main() {
    for input in [
        "2024-01-15T10:30:00+05:30",
        "2024-01-15T10:30:00Z",
        "Jan 15 2024 10:30:00 GMT-0800",
    ] {
        match extract_offset(input) {
            Ok(offset) => println!("{input} -> {offset}"),
            Err(err) => println!("{input} -> error: {err}"),
        }
    }
}
```

Output:

```
2024-01-15T10:30:00+05:30 -> +05:30
2024-01-15T10:30:00Z -> +00:00
Jan 15 2024 10:30:00 GMT-0800 -> -08:00
```

It does not parse or validate the date/time part at all, it only finds the
offset attached to the end.

## CLI usage

```
$ tz-offset-fmt UTC+5:30 -0800 Z bogus
UTC+5:30 -> +05:30
-0800 -> -08:00
Z -> +00:00
bogus -> error: not a recognized offset or timezone abbreviation
```

With no arguments, it reads one offset per line from stdin, which is more
convenient for batch normalization of a file or a pipe:

```
$ cut -f3 access.log | tz-offset-fmt
UTC+5:30 -> +05:30
-0800 -> -08:00
```

Blank lines are skipped. Errors go to stderr per line, same as the
argument form, so one bad line doesn't stop the rest of the batch.

## Status

Early. The parser and its test table live in `src/lib.rs`. No dependencies,
standard library only.
