# Firmware analysis engine (Mission C)

Date: 2026-11-14. Target: iPhone16,2 / D84AP / T8130 / iOS 27.0.1 (24A446), arm64e.

This report describes the static analysis code in `Lab/src/analysis/` and the `homei-lab analyze` command. It runs on files that are already on disk, executes nothing, boots nothing, decrypts nothing and downloads nothing. A passing check shows that a file names the target. It does not show that the target boots, runs or is compatible.

## What it reads and what it writes

| Input | Role | Read how |
|---|---|---|
| Restore archive (IPSW) | `restore archive` | Central directory only. No entry is decompressed. Hashed with SHA-256 |
| `BuildManifest.plist` | `build manifest` | Parsed as XML. Hashed with SHA-256 |
| Extracted tree | IM4P files, decompressed payloads | Headers, keybag elements and payload magic. Payloads hashed |
| Ghidra project (optional) | project metadata | File size, repository presence, a streamed name search. No write |

The command writes two files to the directory it is given: `t8130-firmware-analysis.json` and `t8130-firmware-analysis.md`. The feasibility command reads the JSON and writes `t8130-feasibility.json` and `t8130-feasibility-report.md`.

Every output names files relative to the input root. A test checks that the committed bundle contains no local path, no account name and no owner field.

## Modules

| Module | Purpose | Notes |
|---|---|---|
| `analysis/zipdir.rs` | Central-directory reader, ZIP64 included | The restore archive is 12.4 GB, so ZIP64 is required. Lists 127 entries |
| `analysis/plist.rs` | XML property list subset | Rejects binary plists rather than guessing. Entities decoded |
| `analysis/manifest.rs` | BuildManifest identities, components, identity checks | Erase Customer identity: 97 components. Digests kept as recorded |
| `analysis/apple_dt.rs` | Apple's flattened DeviceTree (little-endian) | Node count, properties, `reg` and `ranges` decoding with declared cell widths. Least-significant cell first |
| `analysis/macho.rs` | Mach-O 64-bit header, segments, fileset entries, build platform | Build UUID deliberately not reported |
| `analysis/im4p.rs` | DER container classification | IM4P (keybag, payload magic), AEA1, UDIF disk, APFS. Headers only |
| `analysis/ghidra_meta.rs` | Ghidra project metadata | Empty-file detection, owner field never read, streamed search of the index. Function counts stay `None` |
| `analysis/confidence.rs` | Claim tracker and confidence tags | `T` target, `M` manifest, `I` inferred, `U` unknown. Only `T` and `M` are facts |
| `analysis/provenance.rs` | Input records and external steps | File name, size and SHA-256 only |
| `analysis/research.rs` | Orchestrator and bundle types | Availability states, component records, claims |
| `analysis/report.rs` | Sanitized Markdown | Same content as the JSON, with confidence tags |
| `feasibility.rs` | T8130 feasibility matrix | Sixteen components, see the feasibility report |

Availability is one of seven states, kept apart on purpose:

- `analysed`: the file is present and its format is parsed.
- `container_only`: the container is recognised, its contents are not parsed.
- `encrypted`: the file has a keybag, and its keys are not in the archive.
- `unsupported`: the format is not recognised.
- `unavailable`: the archive lists the component, but there is no local copy.
- `missing`: the manifest names a path the archive does not contain.
- `no_manifest_path`: the manifest entry states no path, so nothing can be located.

## Result of the final run

Source: `t8130-firmware-analysis.json` in `Data/research/`, generated on 2026-11-14.

| Check | Result |
|---|---|
| Identity checks (product type, build, version, board configuration, chip ID, board ID) | 6 of 6 pass |
| Manifest identities | Erase Customer 97 components; Update Customer 97; Recovery 24 |
| Restore archive SHA-256 | `331bc7088ec5…` (matches the Phase 1 record) |
| Archive entries | 127: 26 stored, 101 deflated; uncompressed total 12,516,836,065 bytes |
| Erase components | 97 total: 28 analysed, 4 encrypted, 61 unavailable (no local copy), 4 no manifest path, 0 missing |
| Manifest digests (SHA-384) against local files | 26 match, 6 do not, 65 not checked (no local copy) |
| Encrypted images (keybag) | `sep-firmware` (`SEP`, `RestoreSEP`, `Yonkers,SepObject`) and `sep-patches` (`SepStage1`) |
| Kernelcache | Mach-O `MH_FILESET`, arm64e, 264 fileset entries, `xnu-13432.2.10~2`, configuration `RELEASE_ARM64_T8122` |
| Device tree | 379 nodes; root compatible `D84AP, iPhone16,2, AppleARM`; `arm-io,t8130` |
| AIC | `aic,3`, 1 interrupt cell, `reg` `0xC1000000` size `0x1CC000`, translated `0x2D1000000` (inferred, first `ranges` entry) |
| PMGR | `pmgr1,t8130`, 83 register windows, 39 power-domain names |
| Ghidra project | Project file is 0 bytes (empty). Index contains `AppleInterruptControllerV3` and `AppleT8130PMGR`. `aic,3` and `pmgr1,t8130` are not in the index. Function count: not exported (unknown) |

Claims in the bundle: nine, each with a tag. The six digest mismatches are all `Restore*` entries. Each shares a path with a non-restore entry whose digest does match, so the restore-variant digests describe a different image. The reason is not established.

## Discrepancies, and what was done about them

1. **Byte order in the DeviceTree reader (fixed).** The first run folded `reg` and `ranges` cells as big-endian. Apple's tree stores the least-significant cell first. The first run therefore produced `0xC1000000 << 24` and no translation. The reader was corrected and covered by a unit test that uses the real layout. The superseded run is kept, not overwritten.
2. **Mach-O addresses above `Int64.max` (fixed in the Swift models).** Kernel segment addresses are `0xFFFFFFF…`. Swift's `Int64` decode rejected the whole kernelcache record with a generic "not valid JSON" error. The Swift field is now `UInt64`, which matches the Rust `u64`.
3. **Four components with no path (fixed).** These are `ftap`, `ftsp`, `rfta` and `rfts`. They were first reported as `missing`. The archive contains nothing missing, so they now have their own state, `no_manifest_path`.
4. **`iBootData` compression (not resolved).** The payload carries the `bvxn` (LZVN) block magic. The `ipsw` inventory records it as LZFSE. The bundle records what the bytes show. The inventory was not edited.
5. **Restore-variant digests (not resolved).** Six `Restore*` manifest digests do not match the local file at the same path. See above.
6. **`aic-iack-offset` (not resolved).** iBoot's string table names it next to `aic`. It is absent from the static DeviceTree. The bundle records both facts.
7. **Kernel configuration token (not resolved).** The kernel names `RELEASE_ARM64_T8122`, while the platform is T8130. This is the known naming discrepancy (tracker D-001). It is kept, not resolved.
8. **Ghidra project file is empty (recorded).** The `.gpr` file is 0 bytes. The repository index is present. No function count exists, and none is reported.

## Verification

| Gate | Result |
|---|---|
| `cargo fmt --check` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass |
| `cargo test --workspace` | 208 passed, 0 failed |
| `cargo deny check` | advisories, bans, licences and sources OK |
| `cargo audit` | 46 crate dependencies scanned, exit 0 |
| `scripts/validate_data.py` (JSON Schema, cross-references, privacy patterns) | All checks passed |
| `swift test` (HomeiKit, Linux, Swift 6.4.1) | 54 passed, 0 failed |

Rust tests that need the real firmware read `HOMEI_T8130_INPUT` and are skipped when it is not set. The committed bundle is tested without it, through `Data/research/`.

## Limitations

- Static analysis of headers and load commands only. No code was disassembled or decompiled, and no function was counted.
- The IM4P payloads were decompressed earlier by an external tool. This code hashes the decompressed files. It does not reproduce the decompression.
- The DeviceTree is the static template from the restore archive. Values that iBoot fills in at boot are absent.
- The 61 unavailable components were not extracted, so their content is not established. The system volume is AEA-encrypted, and its keys are external. No decryption was attempted.
- The analysis does not establish the physical base address of the AIC. It establishes arithmetic from the tree.

## Reproduction

```
homei-lab analyze --ipsw <restore.ipsw> --input <extracted-dir> --ghidra <project.gpr> --out <out-dir> --date 2026-11-14
homei-lab feasibility --analysis <out-dir>/t8130-firmware-analysis.json --out <out-dir> --date 2026-11-14
python3 scripts/export_sptm_census.py <census.csv> <cpreg-gap.json> Data/research/t8130-sptm-census.json
```

The restore archive and the extracted tree are not in this repository. The committed `Data/research/` files are the sanitized outputs.
