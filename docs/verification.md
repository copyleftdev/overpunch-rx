# Verification record

Profile: **Adapted Rust, Power-of-Ten-inspired**. Scope: the library in `src/`,
with parsing/formatting as the operational kernel. Test generators, benchmarks,
fuzzing runtime and example I/O are outside its no-allocation/bounded-loop rules.
The standard `core::fmt` diagnostic adapter delegates to the caller's formatter;
the resource behavior of that formatter is also outside the codec boundary.

This is an implementation evidence record, not strict compliance, an independent
safety audit or certification. The rule mapping documents the implementation's
bounds, contracts and verification scope.

## Evidence

Local date: 2026-11-11. Main toolchain: Rust 1.99.0, x86_64 Linux. Compatibility:
Rust 1.85.0. Miri: 0.1.0 (`a5c825cd82`, 2026-04-14). Source fingerprints are in
[source.sha256](evidence/source.sha256); compiler and platform details are in
[environment.txt](evidence/environment.txt).

| Check | Result / scope |
| --- | --- |
| Formatting | Core, examples/tests, benchmark package and fuzz package |
| Compiler + Clippy | Zero diagnostics for the core/all targets, stable and Rust 1.85.0 |
| Debug/release tests | 14 integration tests plus one doctest |
| MSRV tests | The same test suite on Rust 1.85.0 |
| Exhaustive byte classification | 3,072 combinations: 256 bytes x 3 positions x 2 alphabets x 2 sign policies |
| Independent formatting oracle | 2,217 small signed values compared against standard decimal text formatting |
| Generated valid values | 24,928 encoding/decoding cases across every width, every valid scale and both alphabets |
| Arbitrary data | 20,000 deterministic buffer/coefficient cases, including transactional failure checks |
| Differential validation | 50,000 valid cases checked against an independent implementation |
| Miri | All 10 contract tests pass after the chunk optimization |
| AddressSanitizer / libFuzzer | 11,735,748 executions, 46 seconds, maximum input 128 bytes, no failure found |
| Portability | `no_std` library builds for bare-metal ARM `thumbv7em-none-eabihf` and WebAssembly `wasm32-unknown-unknown` |
| Documentation | `cargo doc --no-deps` with rustdoc warnings denied |
| Package | Local archive builds and verifies; not published to crates.io |

Fuzz executions include inputs rejected at the harness/schema boundary; the count
does not mean every execution reached every codec branch. Miri runs the contract
suite, not all generated cases. Neither establishes a proof of absence of bugs.
Cross-target compilation is not execution on those devices. Hosted CI runs the
checks defined in [ci.yml](../.github/workflows/ci.yml); current results are on the
[Actions page](https://github.com/copyleftdev/overpunch-rx/actions).

Raw evidence: [checks](evidence/checks.log), [Miri](evidence/miri.log),
[fuzz](evidence/fuzz.log), [differential](evidence/differential.log),
[benchmarks](evidence/benchmark.log).

## Rule mapping

`PASS` below is scoped to the stated Rust adaptation and evidence. `WAIVED`
explicitly identifies original C-style rules that this design does not satisfy.
No project approval authority was supplied; these are recorded design deviations,
not approval of strict compliance.

| Rule | Status | Evidence or narrow deviation |
| --- | --- | --- |
| P10-1: simple, nonrecursive control flow | PASS | Manual call review: schema entry points -> bounded helpers; no recursion, async, callbacks or trait objects in codec paths. |
| P10-2: bounded loops | PASS | At most two prefix chunks totaling 37 bytes and output chunks totaling 38; compile-time loops fixed at 38/10. See `design.md`. |
| P10-3: predictable memory | PASS | Rust adaptation permits bounded stack locals. Core has no dependencies or allocator APIs; 38-byte output scratch and immutable tables. Bare-metal build confirms `no_std` compatibility. |
| P10-4: small functions | PASS | All 23 production functions are at most 34 source lines, counting signatures, braces, comments and blanks after rustfmt. |
| P10-5: assertions/contracts | WAIVED | Original two-assertions-per-function density is not met. Public constructors/codec operations return typed errors; private types preserve invariants; `decode_chunk`, `encode_digits`, `encode_chunk` assert internal bounds. Getters, constructors and format helpers do not receive filler assertions. |
| P10-6: narrow scope | PASS | Manual review: private schema/value fields, local working state, immutable compile-time constants, no mutable globals. |
| P10-7: checked inputs/results | PASS | Public ranges/lengths validated, conversions and scaling checked, all fallible operations propagated before output commit. `Result` and `must_use` values; no unwrap/expect in the library. |
| P10-8: restricted metaprogramming | WAIVED | `derive`, `debug_assert`, `assert` and `write!` macros in the listed library modules are retained. No third-party procedural macros, build scripts or feature branches. Debug/release variants tested. |
| P10-9: restricted indirection | WAIVED | Safe Rust slices, references, iterators and statically resolved conversion closures replace literal C pointer rules. `Error::fmt` uses Rust's formatter interface; user formatter behavior is excluded. No raw pointers, unsafe code or user function pointers in the codec. |
| P10-10: strict tools | PASS | Compiler warnings denied, all/pedantic Clippy warnings denied; stable/MSRV checks. Miri and fuzzing are complementary runtime evidence, not additional static analyzers. |

## Remaining limits

- Formal panic-freedom, stack-size and worst-case execution-time proofs are
  **UNVERIFIED**. Bounds have been reasoned about manually and exercised by tests.
- Complete NCPDP/COBOL conformance is **NOT_APPLICABLE** to this numeric-only
  codec. Partner-specific layouts and alternate sign conventions need their own
  integration validation.
- Package metadata points to the public source repository. Compiler, lint and
  local package verification checks are clean.

## Reproduction

Run `sh scripts/check.sh` from the full checkout with the listed toolchains and
targets already installed. Then run the Miri, differential and fuzz commands in
the README. Tooling packages are separate, locked packages; production dependencies
remain empty. No global toolchain settings are changed by the check script.
