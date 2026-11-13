# Reference project audit

Audit date: 2026-11-13. Method: GitHub API metadata, README text, release listings and **directory names only**, plus an HTTP status and page title check of the project websites. No source file was read, no release asset was downloaded, and nothing was cloned, built or run. README and web text were treated as untrusted data.

Because source was not read, this audit does **not** describe how any reference project implements device detection, error reporting or configuration. Those rows are marked "not assessed".

## What was inspected

| Project | Repository | Default branch head | Latest release | Licence |
|---|---|---|---|---|
| Dopamine | `opa334/Dopamine` | `3.x` at `5b1d2c1c3bf5` (committed 2026-11-03) | 3.0.13 (2026-11-03). Assets: an IPA and a TIPA, about 55 MB each. Not downloaded. | MIT, copyright 2023-2024 Lars Fröder |
| Relaxin | `5hux1n/relaxin` | `main` at `7cf367ce6c0d` (committed 2026-09-27) | v0.5.7 (2026-11-01). Assets: an IPA and a TIPA, about 37 MB each. Not downloaded. | None stated |
| unc0ver (original) | `pwn20wndstuff/Undecimus` | `master` at `53ba03f796ef` (committed 2019-07-12) | v5.2.0 (2020-06-09) | BSD-3-Clause, copyright 2019 Pwn20wnd |
| unc0ver (named in the brief) | `Unc0ver-inc/Unc0ver` | `main` at `c12b54ad69fa` (committed 2023-10-30) | None | None stated |

Websites checked: `unc0ver.dev`, `relaxin.me`, `relaxin.dev` and `relaxin.owngoal.dev` all answered HTTP 200.

## Findings per project

### Dopamine (`opa334/Dopamine`)

- Described as a semi-untethered jailbreak. The README first line lists rootless ranges: iOS 15.0 to 17.3.1 (arm64e), 15.0 to 18.7.1, 26.0 to 26.0.1 (A12/A13) and 15.0 to 18.7.1 (arm64). It also states that version-support issues are deleted without response. It lists no iOS 27 range, which matches the research tracker's claim C-133.
- Top level: `Application`, `BaseBin`, `Packages`, `Standalone`, `Makefile`, `sign_and_install.sh`, `sort_localizations.sh`, one CI workflow (`.github/workflows/main.yml`).
- The app directory separates `UI`, `Jailbreak`, `Exploits`, `Extensions`, `Resources` and `App Delegates`, and carries about 25 localisation folders including Korean. The entry point is `main.m`, so the app is Objective-C. The `Exploits` and `BaseBin` contents were deliberately **not** opened.
- Releases ship as both a standard IPA and a TrollStore-style TIPA.
- The upstream tracker audit of this repository is in the research tracker (source S-050).

### Relaxin (`5hux1n/relaxin`)

- The repository contains only a `README.md` and `.gitignore`. Its README says it provides downloads only and is an **unofficial mirror, not affiliated with Relaxin**. It also advertises a paid sponsorship slot in the README.
- It names `relaxin.owngoal.dev` as the official site and `relaxin.dev` as an archive. The `relaxin.me` address from the brief serves the same "Relaxin Jailbreak Archive — IPA / TIPA Downloads & Changelogs" page as `relaxin.dev`. Who operates any of these sites was not verified.
- **No source is published here, so there is nothing to audit architecturally.** The release notes are bilingual (English and Chinese) and mark newer platform support as "experimental".
- The binaries come from an unverified distributor. They were not downloaded, and the project is treated as untrusted.

### unc0ver

- `Unc0ver-inc/Unc0ver` does **not** look like the original project. It was created and last pushed on a single day in 2023, has no releases, and holds screenshots plus a web page stub. Its README promises to "jailbreak any iOS" for free with no computer, and links a chat server. That is the pattern the research tracker's safety policy warns about. It was not used, and it is listed in the notices only to say so.
- The original project is `pwn20wndstuff/Undecimus` (BSD-3-Clause), described as a jailbreak for iOS 11.0 to 12.4. Its default branch was last updated in 2019 and its last release is v5.2.0 (2020). Top-level names include an Xcode project, `Injector`, `apt` and `patchfinder64`. They were not examined.
- `unc0ver.dev` answers HTTP 200 and serves advertising and analytics scripts. Its content was not otherwise assessed, and its ownership was not verified here.

## Topics requested, and what could be said

| Topic | Dopamine | Relaxin | unc0ver | HOMEI approach |
|---|---|---|---|---|
| Application structure | Observed from directory names only: UI, jailbreak logic and exploits are separate folders | No source | Not assessed | Core logic, catalog, bootstrap simulation and UI in separate modules with no privileged code |
| UI architecture | Not assessed (no source read) | No source | Not assessed | SwiftUI over a Foundation-only kit. See `Docs/DESIGN_SPEC.md` |
| Configuration management | Not assessed | No source | Not assessed | Data-driven JSON with JSON Schema validation |
| Device detection | Not assessed | No source | Not assessed | None. Profiles are typed in, imported or bundled fixtures |
| Firmware compatibility reporting | README lists ranges, and says support questions are deleted | Release notes list ranges and "experimental" labels | Not assessed | Evidence-driven engine with five outcomes and cited evidence |
| Error reporting | Not assessed | No source | Not assessed | Explicit unsupported results with reasons |
| Localisation | About 25 languages, with a sort script | Bilingual changelog | Not assessed | English primary, Korean secondary, key parity checked in CI |
| Package catalog presentation | Not assessed | No source | Not assessed | Offline metadata-only catalog |
| Documentation | Short README | README and changelog on a site | Not assessed | Architecture, safety, recovery and compatibility docs |
| Release engineering | CI workflow and dual IPA/TIPA releases | IPA and TIPA releases | Releases up to 2020 | CI-gated tests and validators. No release until a milestone justifies one |
| Licences | MIT | None stated | BSD-3-Clause (original) | MIT code, CC BY 4.0 documentation |
| Recovery information | Not assessed | Not assessed | Not assessed | `Docs/RECOVERY.md` with GO / NO-GO |
| User safety practices | Not assessed | Sponsorship ad slot in a downloads repository | Lookalike repository matches scam patterns | Closed allow-list Safety Manager |

## What was deliberately not done

No exploit, kernel, SPTM, TXM, AMFI or sandbox code was opened, copied, ported or described. No IPA or TIPA was downloaded. No website script was executed. No reference project was forked, renamed or repackaged.
