# Safety

HOMEI is built so that the application itself cannot harm a device. It does not contain the means to.

## What the application can do

Six read-only operations: view compatibility, validate bundled data, export a sanitized diagnostic report, browse the catalog, view a bootstrap simulation, view recovery information.

## What it cannot do, by construction

Everything else is denied with a reason. That includes jailbreak installation, exploit execution, privilege escalation, kernel changes, protected-memory extraction, SPTM/TXM/AMFI bypasses, root filesystem changes, bootstrap or package installation, tweak injection, IPA installation, trust-cache changes, reboots, DFU or Recovery Mode, firmware update, restore or downgrade, update-setting or configuration-profile changes, USB connection, pairing, backups and diagnostic archive creation. Remote scripts are never executed, and unrecognised operation names are denied by default.

External binaries, package sources, profiles and firmware artefacts are rejected. Unverified compatibility never unlocks anything.

## How this is enforced

- **Closed allow-list** in the Safety Manager (Rust and Swift), with tests for every operation.
- **No capability to misuse:** `unsafe` is forbidden. Source-hygiene tests fail the build if product code gains network, process, device or filesystem-write constructs, a build script, or any dependency beyond `serde` and `serde_json`.
- **No fake success:** privileged capabilities return an explicit unsupported result.
- **No jailbreak button:** the interface shows a "Research Only" notice where a button might be.
- **No device code:** no USB, pairing, discovery or polling exists. Profiles are typed in, imported or bundled. The iOS project declares no entitlements, background modes or usage descriptions.
- **Identifier-proof data:** profiles reject unknown keys and identifier-like values, and diagnostic exports refuse identifier-like tokens.
- **Privacy gate in CI:** the validator and the secret scan fail on identifiers, personal paths, tokens and device-state phrases.

## Development rules

The development rules in `CLAUDE.md` apply: no exploit code, no copying from reference projects, no device interaction, no invented evidence. No physical device is ever a test target, and a device in daily use must never be used for experiments. Tests use fixtures.

## Limits of these guarantees

The tests check HOMEI's own source. They say nothing about other software, and a Swift build has not been compiled here, so the Swift side of these guarantees is by design and review, not by a passing build.
