# CLAUDE.md: instructions for AI agents in this repository

HOMEI is an independent, research-stage iOS security framework. It is **not** a working jailbreak. The security findings it displays come from the research tracker (`PalanBanana/ios27-a17pro-jailbreak-tracker`), which stays the source of truth.

## Non-negotiable rules

1. Never write, port, adapt or describe exploit code, kernel read/write primitives, SPTM/TXM/AMFI bypasses, sandbox escapes, privilege escalation, code-signing bypasses, protected-memory extraction, exploit-chain automation or operational jailbreak installation steps.
2. Never copy code, assets, branding or exploit logic from Dopamine, Relaxin, unc0ver or any other jailbreak. Inspect their public documentation and structure only, and record licences in `THIRD_PARTY_NOTICES.md`.
3. Never add a privileged backend, a hidden execution path, a remote-script runner or a shell-from-metadata path. An unavailable capability returns an explicit unsupported result, never a fake success. No fake progress or success animations.
4. Never touch a physical device. Do not run `idevice*`, `usbmuxd` or pairing commands, and do not add USB, pairing, network-discovery or polling code. A device identification needs a separate, explicit user authorisation for that session and is governed by the research tracker's policy, not this repository.
5. Never claim compatibility without exact evidence for the exact device and build. The research target (iPhone16,2 / A17 Pro / iOS 27.0.1 / 24A446) must evaluate to NOT ESTABLISHED until an exact-scope target result and an independent reproduction are on record. Another device, version or chip never transfers.
6. Never invent sources, dates, claims, builds or verdicts. Use `unverified`, `not_established` or `not_researched`.
7. Never publish personal data: no UDID, ECID, serial number, IMEI, EID, Apple Account data, tokens, pairing records, personal paths or real names of private individuals. Never record a maintainer's private device state (installed build, update settings, usage). Fixtures are synthetic and carry the `test_fixture` provenance.
8. Never include private Ghidra databases, firmware images, kernelcaches, function lists or offsets.
9. Keep `README.md` and `README_KR.md` structurally parallel (English is the source of truth), and keep the English and Korean string keys identical.
10. Before finishing any change run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked` and `python3 scripts/validate_data.py`. Fix every failure. Never weaken a check or delete a test to make it pass.
11. Never force-push, rewrite history or delete existing evidence. Supersede a record instead.
12. Do not claim an iOS binary was built unless Xcode produced it. Swift sources that have not been compiled must be described as such.

## Where things live

`Core/` portable logic (Rust). `Catalog/` and `Bootstrap/` metadata-only catalog and simulation. `Data/` evidence, compatibility, research index and fixtures with JSON Schemas. `App/` SwiftUI app and `HomeiKit` (Swift). `Tests/` suites. `Docs/` documentation. See `Docs/ARCHITECTURE.md`.

## Agents

The main agent makes every final decision. Subagents (if used) are read-only, never access a device and never decide compatibility or security conclusions.

## Tone

Plain, precise and appropriately hedged. Say what is not known.
