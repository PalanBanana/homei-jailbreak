# Changelog

All notable changes are recorded here. Dates use ISO 8601.

## [Unreleased]

### 0.2.0 native build and runtime foundation (2026-11-13, branch `v0.2.0-native-build`, not released)

Verified on a GitHub-hosted macOS runner: macOS 26.7.2, Xcode 26.6 (17F113), Swift 6.3.3, iOS Simulator SDK 26.5.

- The Swift code, which had never been compiled, now compiles. `HomeiKit` passes 18 tests. The SwiftUI app builds for the iOS Simulator with code signing disabled (`BUILD SUCCEEDED`, no compiler warnings). 15 simulator UI tests pass. No IPA, no device build, nothing run on a device.
- Rust/Swift parity: 292 hand-written golden vectors checked by both implementations. They exposed real divergences, now fixed (trailing-newline acceptance, byte versus character lengths, unknown keys, open versus closed vocabularies, differing denial reasons).
- UI and accessibility audit with a pixel-level contrast analyzer over every scrolled page: 333 PASS, 0 FAIL, 0 OCCLUDED, 0 CLIPPED, 6 INCONCLUSIVE for the app's own text. Eight Apple audit findings were overruled by pixel evidence and are recorded. Six INCONCLUSIVE results are Apple findings with no element. See `Docs/UI_ACCESSIBILITY_AUDIT.md`.
- Fixed: provenance banner showed a raw key (now localised in English and Korean with a safe fallback); Settings wheel picker truncated its options (now selectable rows); generic card headings (now real names); a profile-switch test that skipped its own tap.
- App icon set generated from the original SVG by a reproducible script (13 opaque PNG sizes, byte-identical on rerun).
- Dependency audit: `cargo audit` and `cargo deny check` pass and run in CI. No third-party dependency version changed. `homei-tests` gained a direct `serde` dependency, which was already in the lockfile, and the five workspace crates moved to version 0.2.0.
- New and changed files: `.github/workflows/swift-build.yml`, `.github/workflows/audit.yml`, `deny.toml`, `Tests/UITests/`, `Tests/golden/`, `Tests/SwiftTests/{ParityTests,LocalizationTests}.swift`, `scripts/{check_contrast,generate_app_icon,summarize_ui_audit}.py`, `Docs/UI_ACCESSIBILITY_AUDIT.md`.
- Still not done: VoiceOver, device testing, Increase Contrast and Reduce Transparency, iPad and landscape.
- The research target still evaluates to NOT ESTABLISHED. No compatibility claim changed.

### Added

- 2026-11-13: initial product foundation (version 0.1.0, not released). No release or tag has been created.
  - Rust workspace: `homei-core` (device profile, evidence, compatibility engine, research index, safety manager, diagnostics, recovery), `homei-catalog`, `homei-bootstrap`, and the `homei` command line tool. 56 tests, formatted and linted with warnings denied.
  - Data: evidence, compatibility declarations, research index and device fixtures, derived from the research tracker (commit `c16b401`, snapshot 2026-11-12), with JSON Schemas and a Python validator.
  - SwiftUI app sources, `HomeiKit` package, English and Korean strings, an original icon and an XcodeGen description. **Not compiled and not built.** No iOS binary exists.
  - Documentation, including a reference project audit and licence notices.
  - CI: validator, formatter, linter, tests and secret scan.
- The research target evaluates to NOT ESTABLISHED. No support is claimed for any device.
