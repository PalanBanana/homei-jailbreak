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

Direct dependencies are `serde` and `serde_json`. The full resolved set, checked with `cargo metadata` on 2026-11-13:

| Crate | Version | Licence |
|---|---|---|
| itoa | 1.0.19 | MIT OR Apache-2.0 |
| memchr | 2.8.3 | Unlicense OR MIT |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 |
| quote | 1.0.47 | MIT OR Apache-2.0 |
| serde | 1.0.229 | MIT OR Apache-2.0 |
| serde_core | 1.0.229 | MIT OR Apache-2.0 |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 |
| serde_json | 1.0.154 | MIT OR Apache-2.0 |
| syn | 3.0.10 | MIT OR Apache-2.0 |
| unicode-ident | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 |
| zmij | 1.0.23 | MIT |

`cargo-audit` and `cargo-deny` were not available when this list was made, so no vulnerability database check was run on these versions.

## Build tooling

The GitHub Actions workflows use `actions/checkout`, `actions/setup-python` and `gitleaks/gitleaks-action`, pinned by commit. The Python validator uses `jsonschema`. These run in CI only and are not distributed.

## Trademarks

Apple, iPhone and iOS are trademarks of Apple Inc. HOMEI is not affiliated with or endorsed by Apple.

## HOMEI's own licences

Application code: MIT ([LICENSE](LICENSE)). Documentation: CC BY 4.0 ([LICENSE-DOCS](LICENSE-DOCS)). The icon and interface design are original work released under the same terms.
