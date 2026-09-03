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

## CLI usage

```
$ tz-offset-fmt UTC+5:30 -0800 Z bogus
UTC+5:30 -> +05:30
-0800 -> -08:00
Z -> +00:00
bogus -> error: not a recognized offset or timezone abbreviation
```

## Status

Early. The parser and its test table live in `src/lib.rs`. No dependencies,
standard library only.
