# Contributing

Thanks for helping. HOMEI is a research-stage framework with strict boundaries, so please read [CLAUDE.md](CLAUDE.md) and [Docs/SAFETY.md](Docs/SAFETY.md) first. The rules there apply to human and AI contributors alike.

## What is welcome

Corrections to logic, data and documentation. Tests. Accessibility and localisation improvements. Better evidence records, with sources, dates and limits. Compiling the Swift code on macOS and reporting what breaks.

## What will not be accepted

Exploit code, kernel or privilege-escalation primitives, SPTM/TXM/AMFI bypasses, jailbreak installation steps, device-connection or pairing code, background discovery, package installation, links to jailbreak downloads, or code and assets copied from other jailbreak projects. Do not submit device identifiers, Apple Account data, pairing records, personal logs or private device state.

## Before opening a pull request

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
python3 scripts/validate_data.py
```

Keep `README.md` and `README_KR.md` parallel and the English and Korean string keys identical. A change that makes the research target anything other than NOT ESTABLISHED needs exact-device, exact-build evidence in the research tracker first.

Maintainers do not run code from pull requests or downloads.
