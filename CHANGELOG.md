# Changelog

All notable changes are recorded here. Dates use ISO 8601.

## [Unreleased]

### Added

- 2026-11-13: initial product foundation (version 0.1.0, not released). No release or tag has been created.
  - Rust workspace: `homei-core` (device profile, evidence, compatibility engine, research index, safety manager, diagnostics, recovery), `homei-catalog`, `homei-bootstrap`, and the `homei` command line tool. 56 tests, formatted and linted with warnings denied.
  - Data: evidence, compatibility declarations, research index and device fixtures, derived from the research tracker (commit `c16b401`, snapshot 2026-11-12), with JSON Schemas and a Python validator.
  - SwiftUI app sources, `HomeiKit` package, English and Korean strings, an original icon and an XcodeGen description. **Not compiled and not built.** No iOS binary exists.
  - Documentation, including a reference project audit and licence notices.
  - CI: validator, formatter, linter, tests and secret scan.
- The research target evaluates to NOT ESTABLISHED. No support is claimed for any device.
