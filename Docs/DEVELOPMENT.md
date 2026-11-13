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

## What was built and tested, and where

| Item | State |
|---|---|
| Rust workspace (core, catalog, bootstrap, CLI) | Built, formatted, linted with `-D warnings` (also with `--all-features`) and tested on the host: 63 tests |
| Python data validator, contrast token check | Run and passing |
| Dependency audit | `cargo audit` (1,509 advisories loaded, no vulnerabilities in 16 crates) and `cargo deny check` (advisories, bans, licences, sources) pass, locally and in the `audit` workflow |
| `HomeiKit` Swift package | Compiled and tested on macOS CI: 18 tests (5 engine, 8 Rust/Swift parity, 5 localisation), 0 failures |
| XcodeGen project and iOS Simulator build | Generated and built on macOS CI, code signing disabled. `BUILD SUCCEEDED`, one tool notice and no compiler warnings |
| Simulator UI tests | 15 tests pass on an iPhone 17 Pro simulator. See [UI_ACCESSIBILITY_AUDIT.md](UI_ACCESSIBILITY_AUDIT.md) |
| Verified toolchain | macOS 26.7.2, Xcode 26.6 (17F113), Swift 6.3.3, iOS Simulator SDK 26.5 (GitHub-hosted `macos-26` image) |
| iOS binary | A Debug simulator `.app` exists only inside CI. **No IPA, no device build, nothing signed, nothing installed on a device** |
| Not done | VoiceOver, device testing, Increase Contrast and Reduce Transparency, iPad and landscape. See the audit document |

The Swift sources are built with `SWIFT_VERSION` 5.0 (Swift 5 language mode) on the Swift 6.3 compiler. Moving to the Swift 6 language mode has not been tried.

## Commands

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo audit && cargo deny check              # cargo install --locked cargo-audit cargo-deny
python3 -m pip install jsonschema && python3 scripts/validate_data.py
python3 scripts/check_contrast.py            # colour tokens against WCAG 4.5:1
python3 scripts/generate_app_icon.py         # rebuilds the icon set from the SVG, byte-for-byte reproducible
cargo run -p homei-cli -- evaluate FIX-A     # also: capabilities, validate, diagnostics, catalog, simulate, recovery, authorize
```

## Native build and tests (macOS with Xcode)

CI does this in `.github/workflows/swift-build.yml`. To do it by hand:

```
swift test --package-path .
xcodegen generate --spec App/project.yml
xcodebuild -list -project App/HOMEI.xcodeproj
xcodebuild build -project App/HOMEI.xcodeproj -scheme HOMEI -configuration Debug \
  -destination 'generic/platform=iOS Simulator' CODE_SIGNING_ALLOWED=NO
xcodebuild test -project App/HOMEI.xcodeproj -scheme HOMEI -destination 'platform=iOS Simulator,name=iPhone 17 Pro' \
  CODE_SIGNING_ALLOWED=NO
```

The project has no signing identity, entitlements, background modes or device capabilities, and nothing here targets a physical device. The Info.plist is generated from build settings, so no plist file is kept in the repository.

## Rust and Swift parity

The Rust core and `HomeiKit` implement the same rules on purpose. `Tests/golden/parity-vectors.json` holds 292 vectors with expected results written by hand. Rust (`Tests/CompatibilityTests/parity.rs`) and Swift (`Tests/SwiftTests/ParityTests.swift`) each have to produce exactly those results. The vectors cover profile validation (including error field and order), identifier detection, compatibility decisions, safety decisions with their reason text, closed vocabularies and strict record decoding. Writing them found real differences, which were fixed: Swift's regular-expression `$` accepted a trailing newline that Rust rejected, character versus byte lengths, and unknown keys that Swift ignored.

## Changing data

Edit `Data/*.json`, then run the validator and the tests. Every compatibility record that cites evidence must reference existing evidence, and no bundled record may be positive. Refresh from the research tracker rather than inventing entries.
