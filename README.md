# overpunch-rx

A Rust library for parsing and encoding signed overpunch and zoned decimal fields
in fixed-width records. Decode ASCII or EBCDIC fields into exact decimal values,
then encode them into caller-owned buffers without allocating memory.

Overpunch stores a number's sign in its final digit. With two implied decimal
places, `1234E` represents **123.45** and `1234N` represents **−123.45**. These
fields appear in pharmacy claims, payroll and other financial data exchanges.

`overpunch-rx` provides explicit field widths, strict byte validation and exact
scaling. Formatting errors leave the output buffer unchanged.

**Rust 1.85+ · `no_std` · Zero dependencies · No unsafe code · MIT**

## Getting started

Add the Git dependency:

```toml
[dependencies]
overpunch-rx = { git = "https://github.com/copyleftdev/overpunch-rx" }
```

```rust
use overpunch_rx::{Encoding, FieldSpec, FixedDecimal};

fn main() -> Result<(), overpunch_rx::Error> {
    let field = FieldSpec::new(8, 2, Encoding::Ascii)?;
    let value = field.decode(b"0001234N")?;
    assert_eq!(value.coefficient(), -12345); // -123.45
    assert_eq!(value.scale(), 2);

    let mut output = [0; 8];
    field.encode(FixedDecimal::new(-12345, 2)?, &mut output)?;
    assert_eq!(&output, b"0001234N");
    Ok(())
}
```

## Field format

| Property | Behavior |
| --- | --- |
| Numeric model | `i128` coefficient and scale; no floating-point conversions |
| Field width | Exactly 1–38 bytes, including the final sign/digit byte |
| Field scale | 0–width; implied decimal point, including purely fractional fields |
| ASCII terminal codes | Positive `{ABCDEFGHI`; negative `}JKLMNOPQR` |
| EBCDIC zoned decimal | Prefix `F0..F9`; terminal `C0..C9` or `D0..D9` |
| Unsigned terminal digits | Rejected unless explicitly enabled |
| Negative zero | Preserved by default; normalization/rejection are explicit policies |
| Rescaling | Exact only; fractional digit loss and integer overflow return errors |
| Output | Full width, zero-padded; every byte remains unchanged on error |
| Resources | No runtime dependencies, `std`, `alloc`, unsafe code, I/O or recursion |

The 38-digit bound is intentional: every magnitude below `10^38` fits in `i128`.
`FixedDecimal::new` accepts all `i128` coefficients, but a field must have enough
capacity to encode them. `i128::MIN` and `i128::MAX` cannot fit a 38-digit field
at unchanged scale and are rejected safely. Exact downscaling may permit an
otherwise too-large coefficient.

`FixedDecimal` uses **representation equality**: `1.0` and `1.00` are distinct,
as are positive and negative zero. Rescale explicitly before comparing compatible
representations. It is a transport value, not an arithmetic or currency type.

## Sign and zero policies

```rust
use overpunch_rx::{Encoding, FieldSpec, SignPolicy, ZeroPolicy};

fn main() -> Result<(), overpunch_rx::Error> {
    let field = FieldSpec::new(8, 2, Encoding::Ascii)?
        .with_sign_policy(SignPolicy::AllowUnsignedPositive)
        .with_zero_policy(ZeroPolicy::NormalizePositive);

    assert_eq!(field.decode(b"00012345")?.coefficient(), 12345);
    Ok(())
}
```

`AllowUnsignedPositive` accepts ordinary ASCII digits or EBCDIC F-zone digits in
the last position. Encoding always emits explicit signed overpunch. Normalization
changes negative zero to positive zero in both directions; rejection returns
`Error::NegativeZero`. No policy accepts embedded signs, whitespace, lowercase
letters, decimal points or mixed alphabets. Errors report a byte and field-relative
offset without copying the entire record.

EBCDIC support is the numeric zoned-decimal alphabet above, not an arbitrary
code-page transcoder. Alternate A/B/E sign nibbles, leading-sign formats,
packed decimal (COMP-3), floating-point conversion and general COBOL PICTURE
parsing are outside this crate's scope. This is not a complete NCPDP validator;
choose policies from the specific file or transaction specification you implement.

## Performance and validation

Each field contains at most 38 bytes. Parsing uses bounded integer operations
and terminal-byte lookup tables; formatting uses a fixed-size stack buffer before
committing the result. No operation allocates memory or uses floating point.

Tests cover documented wire examples, every byte value, numeric boundaries,
exact scaling, negative zero and unchanged output on failure. The library is
also checked with Miri, fuzzing and bare-metal ARM and WebAssembly builds.

See [implementation details](docs/design.md), [benchmarks](docs/performance.md)
and [verification results](docs/verification.md).

## Development

- `cargo test` — published vectors, errors, all byte values, generated cases and doctest.
- `cargo clippy --all-targets -- -D warnings` — strict lint checks.
- `cargo run --example claim_amount` — ASCII-to-EBCDIC example.
- `sh scripts/check.sh` — local checks, including MSRV and embedded targets.
- `cargo +nightly miri test --test contracts` — interpreter checks (Miri required).
- `cargo +nightly fuzz run codec -- -max_total_time=60 -max_len=128` — bounded fuzz run.
- `cargo test --manifest-path benchmarks/Cargo.toml --locked` — differential tests.
- `cargo bench --manifest-path benchmarks/Cargo.toml --bench compare --locked` — benchmarks.

Benchmark and fuzzing tools live in separate packages. The core crate has no
runtime or development dependencies.

Maintainers can use the [release workflow](docs/releasing.md) to verify and publish
new versions to crates.io.

## Format references

Wire examples and test vectors are documented in
the [TPA overpunch map](https://www.tpadministrator.com/internet/tpaw4.nsf/DIDC/6WRPWCLZCE~Coverage%20Gap%20Discount%20Program~References)
and [IBM's NCPDP documentation](https://www.ibm.com/docs/SS6V3G_5.3.1/com.ibm.help.gswhipaancpdp.doc/GSW_Field_Format_Values.html).
