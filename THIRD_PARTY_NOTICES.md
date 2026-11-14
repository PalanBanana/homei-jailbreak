# Third-party notices

HOMEI is independently developed. It is not a fork of, and does not copy code, assets, branding or documentation from, Dopamine, Relaxin, unc0ver or any other jailbreak. Licences were reviewed for everything listed here.

## Reference projects

Inspected as references for high-level product practice only (see [Docs/REFERENCE_PROJECT_AUDIT.md](Docs/REFERENCE_PROJECT_AUDIT.md)). **No code, asset, string or documentation from any of them is included in this repository.**

| Project | Repository inspected | Licence found | Used in HOMEI |
|---|---|---|---|
| Dopamine | `opa334/Dopamine` (MIT, copyright Lars Fröder) | MIT | Nothing. Directory layout and README were read as a reference. |
| Relaxin | `5hux1n/relaxin` (an unofficial downloads-only mirror) | None stated | Nothing. No source is published there. |
| unc0ver | `pwn20wndstuff/Undecimus` | BSD-3-Clause | Nothing. |
| unc0ver (lookalike) | `Unc0ver-inc/Unc0ver` | None stated | Nothing. Treated as untrusted. |

If code from any of these is ever borrowed, its licence text and copyright notice must be added here first, and the change reviewed.

## Research data

`Data/evidence.json` and `Data/research-index.json` are derived from the public research tracker `PalanBanana/ios27-a17pro-jailbreak-tracker` (code MIT, documentation CC BY 4.0, same author). Claim titles are reproduced with attribution. The tracker cites third-party public sources by URL only. Their text is not reproduced.

## Rust dependencies

Direct dependencies of the portable crates are `serde` and `serde_json`. The host controller `homei-lab` adds `sha2` (SHA-256) and `ed25519-dalek` (signature verification), and the test crate uses `homei-lab` and `ed25519-dalek` to build signed fixtures. Resolved with `cargo metadata` on 2026-11-14:

| Crate | Version | Licence |
|---|---|---|
| base64ct | 1.8.3 | Apache-2.0 OR MIT |
| block-buffer | 0.10.4 | MIT OR Apache-2.0 |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 |
| const-oid | 0.9.6 | Apache-2.0 OR MIT |
| cpufeatures | 0.2.17 | MIT OR Apache-2.0 |
| crypto-common | 0.1.7 | MIT OR Apache-2.0 |
| curve25519-dalek | 4.1.3 | BSD-3-Clause |
| curve25519-dalek-derive | 0.1.1 | MIT/Apache-2.0 |
| der | 0.7.10 | Apache-2.0 OR MIT |
| digest | 0.10.7 | MIT OR Apache-2.0 |
| ed25519 | 2.2.3 | Apache-2.0 OR MIT |
| ed25519-dalek | 2.2.0 | BSD-3-Clause |
| fiat-crypto | 0.2.9 | MIT OR Apache-2.0 OR BSD-1-Clause |
| generic-array | 0.14.7 | MIT |
| getrandom | 0.2.17 | MIT OR Apache-2.0 |
| itoa | 1.0.19 | MIT OR Apache-2.0 |
| libc | 0.2.191 | MIT OR Apache-2.0 |
| memchr | 2.8.3 | Unlicense OR MIT |
| pkcs8 | 0.10.2 | Apache-2.0 OR MIT |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 |
| quote | 1.0.47 | MIT OR Apache-2.0 |
| rand_core | 0.6.4 | MIT OR Apache-2.0 |
| rustc_version | 0.4.1 | MIT OR Apache-2.0 |
| semver | 1.0.28 | MIT OR Apache-2.0 |
| serde | 1.0.229 | MIT OR Apache-2.0 |
| serde_core | 1.0.229 | MIT OR Apache-2.0 |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 |
| serde_json | 1.0.154 | MIT OR Apache-2.0 |
| sha2 | 0.10.9 | MIT OR Apache-2.0 |
| signature | 2.2.0 | Apache-2.0 OR MIT |
| spki | 0.7.3 | Apache-2.0 OR MIT |
| subtle | 2.6.1 | BSD-3-Clause |
| syn | 2.0.119 and 3.0.10 | MIT OR Apache-2.0 |
| typenum | 1.20.1 | MIT OR Apache-2.0 |
| unicode-ident | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 |
| version_check | 0.9.5 | MIT/Apache-2.0 |
| wasi | 0.11.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| zeroize | 1.9.1 | Apache-2.0 OR MIT |
| zmij | 1.0.23 | MIT |

`cargo audit` (1,509 advisories loaded) found no vulnerabilities in the 46 crates of `Cargo.lock` on 2026-11-14, and `cargo deny check` passes the licence, ban and source policy in `deny.toml`. That policy gained one permissive licence, `BSD-3-Clause`, for `curve25519-dalek`, `ed25519-dalek` and `subtle`. No copyleft licence is allowed.

## Referenced upstream projects (not included)

`jprx/darwin-vm` (MIT) and its `qemu-sptm` fork of QEMU (GPL-2.0-or-later, reported as `NOASSERTION` by the hosting service) are referenced by pinned commit and described in `Docs/DARWIN_VM_BACKEND.md`. HOMEI does not vendor, modify or redistribute either one, and does not copy their text. Ghidra (Apache-2.0) and `ipsw` (MIT) are used by the researcher as external tools and are not distributed.

## Build tooling

The GitHub Actions workflows use `actions/checkout`, `actions/setup-python`, `actions/upload-artifact` and `gitleaks/gitleaks-action`, pinned by commit. CI also uses XcodeGen (MIT, installed with Homebrew when the runner does not have it), `cargo-audit` and `cargo-deny` (pinned versions 0.23.0 and 0.20.2, both Apache-2.0 or MIT licensed). The Python validator uses `jsonschema`. The icon is rendered with librsvg and ImageMagick. These are build and test tools only and are not distributed.

## Trademarks

Apple, iPhone and iOS are trademarks of Apple Inc. HOMEI is not affiliated with or endorsed by Apple.

## HOMEI's own licences

Application code: MIT ([LICENSE](LICENSE)). Documentation: CC BY 4.0 ([LICENSE-DOCS](LICENSE-DOCS)). The icon and interface design are original work released under the same terms.
