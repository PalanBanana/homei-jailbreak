# HOMEI Jailbreak

**Independent iOS Security and Jailbreak Framework** · codename HOMEI-A17

> **HOMEI is a research-stage framework. It is not a working jailbreak, it contains no exploit, and it does not connect to or modify any device.** It does not claim support for any device. For the research target below, compatibility is **NOT ESTABLISHED**.

[한국어](README_KR.md)

## 1. Status

| Item | State |
|---|---|
| Version | 0.3.0 (A17 Pro virtual research lab milestone, not released) |
| Target compatibility | **NOT ESTABLISHED** |
| Readiness | LEVEL 0 / **NO-GO** |
| Jailbreak backend | None |
| Rust core, catalog, bootstrap simulation, CLI, lab host controller | Built and tested on the host (172 tests, 109 of them for the lab), dependency audit clean |
| Virtual research lab | Host controller, guest storage, package manager, experiments and ten read-only app screens. **No guest was booted.** VM status `BLOCKED_BY_MISSING_QEMU_FEATURE`, hardware fidelity `UNVERIFIED`. No exact A17 Pro VM is claimed |
| Swift package (`HomeiKit`) | 0.2.0: compiled and tested on macOS CI (18 tests, 8 of them Rust/Swift parity). The 0.3.0 lab models and tests are written on a Linux host that has no Swift toolchain, so only the macOS CI run checks them |
| SwiftUI app | Builds for the iOS Simulator with code signing disabled (Xcode 26.6, SDK 26.5). 15 simulator UI tests pass |
| iOS binary | Simulator `.app` only, built in CI. **No IPA, no device build, nothing run on a device** |

## 2. What HOMEI is

An evidence-driven compatibility engine, an offline research index tied to a public research tracker, a closed-allow-list Safety Manager, a metadata-only package catalog, a bootstrap planning simulator, recovery documentation, an original SwiftUI interface in English and Korean, and, since 0.3.0, a **virtual research lab**: a host controller that can describe, prepare and (when explicitly told to) run a Darwin guest for the research target, with isolated guest storage, snapshots, firmware and Ghidra metadata, a VM-only bootstrap, a guest-only package manager and experiment records. The lab is not an iPhone emulator and does not claim hardware fidelity. See [Docs/VIRTUAL_RESEARCH_LAB.md](Docs/VIRTUAL_RESEARCH_LAB.md). It is independently developed. It is not a fork of Dopamine, Relaxin, unc0ver or any other jailbreak, and it copies nothing from them.

## 3. What HOMEI is not

Not an exploit, a kernel or SPTM bypass, a bootstrap that installs, a package manager that installs, or a device tool. No USB, pairing, discovery or polling code exists, in the app or in the lab host controller. Privileged capabilities return an explicit unsupported result, never a fake success. It is not affiliated with or endorsed by Apple or any jailbreak project.

## 4. Research target

| Field | Value |
|---|---|
| Device | iPhone 15 Pro Max (`iPhone16,2`) |
| SoC | Apple A17 Pro |
| Firmware | iOS 27.0.1, build 24A446 |
| Architecture | arm64e |

The target is a public research profile. A result for another device, version or chip never transfers to it. The security findings come from the research tracker, which remains the source of truth: [ios27-a17pro-jailbreak-tracker](https://github.com/PalanBanana/ios27-a17pro-jailbreak-tracker).

## 5. Architecture

Modules: device profile, compatibility engine, evidence database, research intelligence, safety manager, diagnostics, package catalog, bootstrap planning, recovery, and privileged capability status. The portable core is Rust, validation is Python and JSON Schema, and the interface is SwiftUI over a Foundation-only kit. The lab adds the `Lab` crate (the only crate that writes files or starts a process, through a path guard and a two-program allow-list). See [Docs/ARCHITECTURE.md](Docs/ARCHITECTURE.md).

## 6. Build and test

```
cargo test --workspace --locked
python3 -m pip install jsonschema && python3 scripts/validate_data.py
cargo run -p homei-cli -- evaluate FIX-A
cargo run -p homei-lab -- preflight          # reads the host environment, starts nothing
```

The Swift package and the app build and test on macOS with Xcode. CI does this on every push, for the simulator only. See [Docs/DEVELOPMENT.md](Docs/DEVELOPMENT.md) and [Docs/UI_ACCESSIBILITY_AUDIT.md](Docs/UI_ACCESSIBILITY_AUDIT.md).

## 7. Safety

The application can only view, validate, export a sanitized report, browse metadata and run a simulation. Everything else is denied. Source-hygiene tests fail the build if the portable crates gain network, process, device or filesystem-write constructs, and they pin the lab host controller's allow-list, dependencies and write path. See [Docs/SAFETY.md](Docs/SAFETY.md) and [Docs/RECOVERY.md](Docs/RECOVERY.md). Never use a daily-use device as a test target.

## 8. Reference projects

Dopamine, Relaxin and unc0ver were inspected for high-level product practice only, from repository metadata, READMEs and directory names. No code or assets were used. Two of the three references did not match what was expected: the Relaxin repository is an unofficial downloads-only mirror with no source, and `Unc0ver-inc/Unc0ver` is a lookalike, not the original project. See [Docs/REFERENCE_PROJECT_AUDIT.md](Docs/REFERENCE_PROJECT_AUDIT.md) and [Docs/PRODUCT_DIFFERENTIATION.md](Docs/PRODUCT_DIFFERENTIATION.md).

## 9. Documentation

[Product requirements](Docs/PRODUCT_REQUIREMENTS.md) · [Architecture](Docs/ARCHITECTURE.md) · [Virtual research lab](Docs/VIRTUAL_RESEARCH_LAB.md) · [Exact target feasibility](Docs/EXACT_TARGET_FEASIBILITY.md) · [Darwin VM backend](Docs/DARWIN_VM_BACKEND.md) · [VM fidelity](Docs/VM_FIDELITY.md) · [Ghidra integration](Docs/GHIDRA_INTEGRATION.md) · [Firmware manifest](Docs/FIRMWARE_MANIFEST.md) · [Guest bootstrap](Docs/VIRTUAL_BOOTSTRAP.md) · [Guest package manager](Docs/VIRTUAL_PACKAGE_MANAGER.md) · [Experiments](Docs/EXPERIMENTS.md) · [Privacy](Docs/PRIVACY.md) · [Device compatibility](Docs/DEVICE_COMPATIBILITY.md) · [Safety](Docs/SAFETY.md) · [Recovery](Docs/RECOVERY.md) · [Design specification](Docs/DESIGN_SPEC.md) · [UI and accessibility audit](Docs/UI_ACCESSIBILITY_AUDIT.md) · [Development](Docs/DEVELOPMENT.md) · [Roadmap](Docs/ROADMAP.md) · [Third-party notices](THIRD_PARTY_NOTICES.md) · [Contributing](CONTRIBUTING.md) · [Security](SECURITY.md)

## 10. License

Code: MIT ([LICENSE](LICENSE)). Documentation: CC BY 4.0 ([LICENSE-DOCS](LICENSE-DOCS)). Apple, iPhone and iOS are trademarks of Apple Inc.
