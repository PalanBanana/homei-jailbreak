# Development

## Host toolchain

Discovered on 2026-11-13 on the Linux development host:

| Tool | Version |
|---|---|
| Rust / Cargo | 1.99.1 |
| Clang | 23.1.4 |
| CMake | 4.4.5 |
| Python | 3.14.9 |
| Git / GitHub CLI | 2.56.0 / 2.103.3 |
| Java | OpenJDK 25 |
| Swift, Xcode, iOS SDK | **not installed** |

Ghidra exists on the host, and HOMEI does not use it. `cargo-audit` and `cargo-deny` were not installed.

## What was built and tested here, and what was not

| Item | State |
|---|---|
| Rust workspace (core, catalog, bootstrap, CLI) | Built, formatted, linted with `-D warnings` and tested (56 tests) |
| Python data validator | Run and passing in a virtualenv with `jsonschema` |
| JSON Schemas, XML resources | Validated |
| Swift sources (`App/`, `Package.swift`, `Tests/SwiftTests/`) | **Written but never compiled.** Only a bracket-balance check was run. They may contain compile errors. |
| Xcode project | **Not generated.** `App/project.yml` is a description for XcodeGen |
| iOS binary or IPA | **Does not exist.** None was built, and none is claimed |

## Commands

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
python3 -m pip install jsonschema && python3 scripts/validate_data.py
cargo run -p homei-cli -- evaluate FIX-A     # also: capabilities, validate, diagnostics, catalog, simulate, recovery, authorize
```

## Building the iOS app (needs macOS with Xcode)

1. `swift test` at the repository root. This exercises `HomeiKit` against `Data/`. It is the first thing to run, and fixing any compile errors is expected.
2. `xcodegen generate --spec App/project.yml`, open the project and build for the simulator.
3. Record the result in `CHANGELOG.md`. Do not describe the app as built until this succeeds.

Do not run the app on a daily-use device as an experiment. Use the simulator.

## Changing data

Edit `Data/*.json`, then run the validator and the tests. Every compatibility record that cites evidence must reference existing evidence, and no bundled record may be positive. Refresh from the research tracker rather than inventing entries.
