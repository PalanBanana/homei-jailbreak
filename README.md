# HOMEI Jailbreak

**Independent iOS Security and Jailbreak Framework** · codename HOMEI-A17

> **HOMEI is a research-stage framework. It is not a working jailbreak, it contains no exploit, and it does not connect to or modify any device.** It does not claim support for any device. For the research target below, compatibility is **NOT ESTABLISHED**.

[한국어](README_KR.md)

## 1. Status

| Item | State |
|---|---|
| Version | 0.1.0 (initial foundation, not released) |
| Target compatibility | **NOT ESTABLISHED** |
| Readiness | LEVEL 0 / **NO-GO** |
| Jailbreak backend | None |
| Rust core, catalog, bootstrap simulation, CLI | Built and tested on the host (56 tests) |
| Swift package and SwiftUI app | Written, **not compiled** (no Swift or Xcode in the development environment) |
| iOS binary | **None built** |

## 2. What HOMEI is

An evidence-driven compatibility engine, an offline research index tied to a public research tracker, a closed-allow-list Safety Manager, a metadata-only package catalog, a bootstrap planning simulator, recovery documentation, and an original SwiftUI interface in English and Korean. It is independently developed. It is not a fork of Dopamine, Relaxin, unc0ver or any other jailbreak, and it copies nothing from them.

## 3. What HOMEI is not

Not an exploit, a kernel or SPTM bypass, a bootstrap that installs, a package manager that installs, or a device tool. No USB, pairing, discovery or polling code exists. Privileged capabilities return an explicit unsupported result, never a fake success. It is not affiliated with or endorsed by Apple or any jailbreak project.

## 4. Research target

| Field | Value |
|---|---|
| Device | iPhone 15 Pro Max (`iPhone16,2`) |
| SoC | Apple A17 Pro |
| Firmware | iOS 27.0.1, build 24A446 |
| Architecture | arm64e |

The target is a public research profile. A result for another device, version or chip never transfers to it. The security findings come from the research tracker, which remains the source of truth: [ios27-a17pro-jailbreak-tracker](https://github.com/PalanBanana/ios27-a17pro-jailbreak-tracker).

## 5. Architecture

Modules: device profile, compatibility engine, evidence database, research intelligence, safety manager, diagnostics, package catalog, bootstrap planning, recovery, and privileged capability status. The portable core is Rust, validation is Python and JSON Schema, and the interface is SwiftUI over a Foundation-only kit. See [Docs/ARCHITECTURE.md](Docs/ARCHITECTURE.md).

## 6. Build and test

```
cargo test --workspace --locked
python3 -m pip install jsonschema && python3 scripts/validate_data.py
cargo run -p homei-cli -- evaluate FIX-A
```

The Swift sources and the iOS app need macOS with Xcode and have not been built. See [Docs/DEVELOPMENT.md](Docs/DEVELOPMENT.md).

## 7. Safety

The application can only view, validate, export a sanitized report, browse metadata and run a simulation. Everything else is denied. Source-hygiene tests fail the build if product code gains network, process, device or filesystem-write constructs. See [Docs/SAFETY.md](Docs/SAFETY.md) and [Docs/RECOVERY.md](Docs/RECOVERY.md). Never use a daily-use device as a test target.

## 8. Reference projects

Dopamine, Relaxin and unc0ver were inspected for high-level product practice only, from repository metadata, READMEs and directory names. No code or assets were used. Two of the three references did not match what was expected: the Relaxin repository is an unofficial downloads-only mirror with no source, and `Unc0ver-inc/Unc0ver` is a lookalike, not the original project. See [Docs/REFERENCE_PROJECT_AUDIT.md](Docs/REFERENCE_PROJECT_AUDIT.md) and [Docs/PRODUCT_DIFFERENTIATION.md](Docs/PRODUCT_DIFFERENTIATION.md).

## 9. Documentation

[Product requirements](Docs/PRODUCT_REQUIREMENTS.md) · [Architecture](Docs/ARCHITECTURE.md) · [Device compatibility](Docs/DEVICE_COMPATIBILITY.md) · [Safety](Docs/SAFETY.md) · [Recovery](Docs/RECOVERY.md) · [Design specification](Docs/DESIGN_SPEC.md) · [Development](Docs/DEVELOPMENT.md) · [Roadmap](Docs/ROADMAP.md) · [Third-party notices](THIRD_PARTY_NOTICES.md) · [Contributing](CONTRIBUTING.md) · [Security](SECURITY.md)

## 10. License

Code: MIT ([LICENSE](LICENSE)). Documentation: CC BY 4.0 ([LICENSE-DOCS](LICENSE-DOCS)). Apple, iPhone and iOS are trademarks of Apple Inc.
