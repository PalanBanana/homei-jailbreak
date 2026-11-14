# T8130 firmware analysis

Target: **iPhone16,2 / D84AP / T8130 / iOS 27.0.1 (24A446)**. Analysed on 2026-11-14. Schema 1. Tool `homei-lab analyze` 0.3.0.

This is a static analysis of firmware files. Nothing was executed, booted or emulated. A passing check shows that the file names the target. It does not show that the target boots, runs or is compatible.

## Target identity

| Check | Expected | Observed | Result |
|---|---|---|---|
| product_type_supported | iPhone16,2 | iPhone16,2 | PASS |
| build_version | 24A446 | 24A446 | PASS |
| product_version | 27.0.1 | 27.0.1 | PASS |
| board_config | d84ap | d84ap | PASS |
| chip_id | 0x8130 | 0x8130 | PASS |
| board_id | 0x06 | 0x06 | PASS |

## Restore archive

- Entries in the central directory: 127 (26 stored, 101 deflated)
- Uncompressed total: 12516836065 bytes (listing only; no entry was decompressed)
- Erase components named by the manifest but absent from the archive: 0

## Components (erase identity)

97 components: 28 analysed, 0 container only, 4 encrypted, 0 unsupported, 61 unavailable, 0 missing, 4 with no path stated.

| Component | Availability | Container | SHA-384 matches manifest | Payload SHA-256 |
|---|---|---|---|---|
| ANE | analysed | IM4P `anef` | yes | 885e1e0fd2b23135… |
| ANS | analysed | IM4P `ansf` | yes | 140735c86526eb89… |
| AOP | analysed | IM4P `aopf` | yes | 2c19e1ea938773d2… |
| AVE | analysed | IM4P `avef` | yes | 8770e9002c35c723… |
| Ap,AudioBootChime | unavailable (no local copy) | - | not checked | - |
| Ap,HapticAssets | unavailable (no local copy) | - | not checked | - |
| Ap,LowPowerMode | analysed | IM4P `alpm` | yes | 0674116b3d2a7ea0… |
| Ap,RestoreSecurePageTableMonitor | analysed | IM4P `sptm` | no | 057ebf185f43c6ee… |
| Ap,RestoreTrustedExecutionMonitor | analysed | IM4P `trxm` | no | e058931840ac000f… |
| Ap,SecurePageTableMonitor | analysed | IM4P `sptm` | yes | 057ebf185f43c6ee… |
| Ap,SystemVolumeCanonicalMetadata | unavailable (no local copy) | - | not checked | - |
| Ap,TrustedExecutionMonitor | analysed | IM4P `trxm` | yes | e058931840ac000f… |
| AppleLogo | unavailable (no local copy) | - | not checked | - |
| BMU,DigestMap | unavailable (no local copy) | - | not checked | - |
| BMU,FirmwareMap | unavailable (no local copy) | - | not checked | - |
| BasebandFirmware | unavailable (no local copy) | - | not checked | - |
| Cryptex1,AppOS | unavailable (no local copy) | - | not checked | - |
| Cryptex1,AppTrustCache | unavailable (no local copy) | - | not checked | - |
| Cryptex1,AppVolume | unavailable (no local copy) | - | not checked | - |
| Cryptex1,SystemOS | unavailable (no local copy) | - | not checked | - |
| Cryptex1,SystemTrustCache | unavailable (no local copy) | - | not checked | - |
| Cryptex1,SystemVolume | unavailable (no local copy) | - | not checked | - |
| DCP | analysed | IM4P `dcpf` | yes | ef9745b548ff730b… |
| DeviceTree | analysed | IM4P `dtre` | yes | 684df728dd257c2e… |
| GFX | analysed | IM4P `gfxf` | yes | 1cc0bab608c8353d… |
| ISP | unavailable (no local copy) | - | not checked | - |
| KernelCache | analysed | IM4P `krnl` | yes | 71d3232aff357cf0… |
| LLB | analysed | IM4P `illb` | yes | 29082c277a0688c3… |
| LeapHaptics | unavailable (no local copy) | - | not checked | - |
| Liquid | unavailable (no local copy) | - | not checked | - |
| MtpFirmware | unavailable (no local copy) | - | not checked | - |
| Multitouch | unavailable (no local copy) | - | not checked | - |
| OS | unavailable (no local copy) | - | not checked | - |
| PMP | analysed | IM4P `pmpf` | yes | 9e1ad315804123f5… |
| Rap,RTKitIOConfig | unavailable (no local copy) | - | not checked | - |
| Rap,RTKitOS | unavailable (no local copy) | - | not checked | - |
| Rap,SoftwareBinaryDsp1 | unavailable (no local copy) | - | not checked | - |
| RecoveryMode | unavailable (no local copy) | - | not checked | - |
| RestoreANS | analysed | IM4P `rans` | yes | 140735c86526eb89… |
| RestoreDCP | analysed | IM4P `dcpf` | no | 50f59d3c511add08… |
| RestoreDeviceTree | analysed | IM4P `dtre` | no | 684df728dd257c2e… |
| RestoreKernelCache | analysed | IM4P `krnl` | no | 71d3232aff357cf0… |
| RestoreLogo | unavailable (no local copy) | - | not checked | - |
| RestoreRamDisk | analysed | IM4P `rdsk` | yes | - |
| RestoreSEP | encrypted (keys not available) | IM4P `sepi` | no | 68cf267287cbb919… |
| RestoreTrustCache | analysed | IM4P `rtsc` | yes | - |
| SE,RapRTKitOS | unavailable (no local copy) | - | not checked | - |
| SE,RapSwBinDsp | unavailable (no local copy) | - | not checked | - |
| SE,UpdatePayload | unavailable (no local copy) | - | not checked | - |
| SEP | encrypted (keys not available) | IM4P `sepi` | yes | 68cf267287cbb919… |
| SIO | analysed | IM4P `siof` | yes | 3ae134fd6465d40b… |
| Savage,B0-Dev-Patch | unavailable (no local copy) | - | not checked | - |
| Savage,B0-Dev-PatchVT | unavailable (no local copy) | - | not checked | - |
| Savage,B0-Prod-Patch | unavailable (no local copy) | - | not checked | - |
| Savage,B0-Prod-PatchVT | unavailable (no local copy) | - | not checked | - |
| Savage,B2-Dev-Patch | unavailable (no local copy) | - | not checked | - |
| Savage,B2-Dev-PatchVT | unavailable (no local copy) | - | not checked | - |
| Savage,B2-Prod-Patch | unavailable (no local copy) | - | not checked | - |
| Savage,B2-Prod-PatchVT | unavailable (no local copy) | - | not checked | - |
| Savage,BA-Dev-Patch | unavailable (no local copy) | - | not checked | - |
| Savage,BA-Prod-Patch | unavailable (no local copy) | - | not checked | - |
| Savage,BE-Dev-Patch | unavailable (no local copy) | - | not checked | - |
| Savage,BE-Prod-Patch | unavailable (no local copy) | - | not checked | - |
| Savage,BF-Dev-Patch | unavailable (no local copy) | - | not checked | - |
| Savage,BF-Prod-Patch | unavailable (no local copy) | - | not checked | - |
| SepStage1 | encrypted (keys not available) | IM4P `stg1` | yes | 4d130a6cbefb2864… |
| StaticTrustCache | analysed | IM4P `trst` | yes | - |
| SystemVolume | analysed | IM4P `isys` | yes | - |
| USBPortController1,USBFirmware | unavailable (no local copy) | - | not checked | - |
| WCHFirmwareUpdater | unavailable (no local copy) | - | not checked | - |
| Yonkers,SepObject | encrypted (keys not available) | IM4P `sepi` | yes | 68cf267287cbb919… |
| Yonkers,SysTopPatch0 | unavailable (no local copy) | - | not checked | - |
| Yonkers,SysTopPatch1 | unavailable (no local copy) | - | not checked | - |
| Yonkers,SysTopPatch2 | unavailable (no local copy) | - | not checked | - |
| Yonkers,SysTopPatch3 | unavailable (no local copy) | - | not checked | - |
| Yonkers,SysTopPatch4 | unavailable (no local copy) | - | not checked | - |
| Yonkers,SysTopPatch5 | unavailable (no local copy) | - | not checked | - |
| Yonkers,SysTopPatch6 | unavailable (no local copy) | - | not checked | - |
| Yonkers,SysTopPatch7 | unavailable (no local copy) | - | not checked | - |
| Yonkers,SysTopPatch8 | unavailable (no local copy) | - | not checked | - |
| Yonkers,SysTopPatch9 | unavailable (no local copy) | - | not checked | - |
| Yonkers,SysTopPatchA | unavailable (no local copy) | - | not checked | - |
| Yonkers,SysTopPatchB | unavailable (no local copy) | - | not checked | - |
| Yonkers,SysTopPatchC | unavailable (no local copy) | - | not checked | - |
| Yonkers,SysTopPatchD | unavailable (no local copy) | - | not checked | - |
| Yonkers,SysTopPatchE | unavailable (no local copy) | - | not checked | - |
| Yonkers,SysTopPatchF | unavailable (no local copy) | - | not checked | - |
| eUICC,Gold | unavailable (no local copy) | - | not checked | - |
| eUICC,Main | unavailable (no local copy) | - | not checked | - |
| ftap | no path stated in manifest | - | not checked | - |
| ftsp | no path stated in manifest | - | not checked | - |
| iBEC | analysed | IM4P `ibec` | yes | 29082c277a0688c3… |
| iBSS | analysed | IM4P `ibss` | yes | 29082c277a0688c3… |
| iBoot | analysed | IM4P `ibot` | yes | 29082c277a0688c3… |
| iBootData | analysed | IM4P `ibdt` | yes | 3d233d0bb33171f2… |
| rfta | no path stated in manifest | - | not checked | - |
| rfts | no path stated in manifest | - | not checked | - |

## Security and coprocessor images

| Role | Component | Availability | Encryption | Payload compression |
|---|---|---|---|---|
| SEP firmware | RestoreSEP | encrypted (keys not available) | keybag | none |
| SEP firmware | SEP | encrypted (keys not available) | keybag | none |
| SEP firmware | SepStage1 | encrypted (keys not available) | keybag | none |
| SEP firmware | Yonkers,SepObject | encrypted (keys not available) | keybag | none |
| restore ramdisk | RestoreRamDisk | analysed | not_encrypted | none |
| secure page table monitor | Ap,RestoreSecurePageTableMonitor | analysed | not_encrypted | lzfse |
| secure page table monitor | Ap,SecurePageTableMonitor | analysed | not_encrypted | lzfse |
| trusted execution monitor | Ap,RestoreTrustedExecutionMonitor | analysed | not_encrypted | lzfse |
| trusted execution monitor | Ap,TrustedExecutionMonitor | analysed | not_encrypted | lzfse |

## Kernelcache

- Payload: 73023488 bytes
- Mach-O: arm64e MH_FILESET, cputype 0x0100000c, cpusubtype 0xc0000002, 277 load commands
- Segments: __TEXT, __PRELINK_TEXT, __DATA_CONST, __DATA_SPTM, __TEXT_EXEC, __TEXT_BOOT_EXEC, __PRELINK_INFO, __DATA, __LINKEDIT
- Fileset entries: 264
- Build platform 0, minimum OS 0.0.0
- XNU version string: `xnu-13432.2.10~2`
- Kernel configuration token: `RELEASE_ARM64_T8122`

## Device tree

- Nodes: 379
- Root compatible: D84AP, iPhone16,2, AppleARM
- Root model: iPhone16,2
- Target type: D84
- arm-io compatible: arm-io,t8130
- arm-io range: child 0x0 to parent 0x210000000 size 0x140000000
- arm-io range: child 0x700000000 to parent 0x700000000 size 0x380000000
- arm-io range: child 0x580000000 to parent 0x580000000 size 0x80000000
- arm-io range: child 0x5a0000000 to parent 0x5a0000000 size 0x20000000
- arm-io range: child 0x5c0000000 to parent 0x5c0000000 size 0x40000000
- arm-io range: child 0x600000000 to parent 0x600000000 size 0x80000000

### Interrupt controller

- Path /arm-io/aic, compatible aic,3
- Interrupt cells: 1
- reg: address 0xc1000000, size 0x1cc000 (relative to arm-io)
- reg-names present: false
- Translated base: 0x2d1000000 (first /arm-io ranges entry containing the AIC window)
- Consumers with one interrupt cell: 52; with several elements: 34
- aic-iack-offset in the tree: false

### Power manager

- Path /arm-io/pmgr, compatible pmgr1,t8130
- Register windows: 83
- Power-domain names found in the blob: 39

## Ghidra project metadata

- Project: HOMEI_A17Pro_iOS27_0_1_24A446
- Project file: empty (0 bytes)
- Repository present: true; index files scanned: 9
- `AppleInterruptControllerV3` in index: true
- `AppleT8130PMGR` in index: true
- `aic,3` in index: false
- `pmgr1,t8130` in index: false
- Function count: not exported (unknown)

## Claims

| ID | Tag | Statement | Source | Limitation |
|---|---|---|---|---|
| FA-001 | [T] | The restore archive holds 127 entries, and 0 erase components are missing from it | restore archive central directory | Listing only. Entry contents were not read or decompressed |
| FA-002 | [T] | 26 of 32 manifest digests match the local file; 6 do not: Ap,RestoreSecurePageTableMonitor, Ap,RestoreTrustedExecutionMonitor, RestoreDCP, RestoreDeviceTree, RestoreKernelCache, RestoreSEP | manifest Digest (SHA-384) compared with the local file of the same path | The mismatching entries name the same path as a non-restore entry. The reason for the difference is not established |
| FA-003 | [T] | 4 components carry a keybag and are not analysed: RestoreSEP, SEP, SepStage1, Yonkers,SepObject | IM4P keybag element of each local file | The keys are not in the restore archive. No decryption was attempted |
| FA-004 | [M] | 4 erase components state no file path in the manifest: ftap, ftsp, rfta, rfts | BuildManifest.plist, erase identity | Without a path they cannot be located in the archive. Their content is not established |
| FA-005 | [T] | The kernel payload is a Mach-O arm64e MH_FILESET image with 264 fileset entries | decompressed kernel payload, Mach-O load commands | Header and load commands only. No code was analysed |
| FA-006 | [T] | The device tree has 379 nodes; its root compatible is D84AP, iPhone16,2, AppleARM | decompressed device tree payload (binary) | Static template. Values that iBoot fills at boot are not present |
| FA-007 | [T] | The AIC node has compatible aic,3 and 1 interrupt cell(s) | device tree /arm-io/aic | The one-cell convention is Apple's (see the kernel diagnostic claim) |
| FA-008 | [I] | The AIC window translates to 0x2d1000000 through the first /arm-io ranges entry | device tree /arm-io/ranges and /arm-io/aic reg, cell widths from the nodes | The arithmetic is established. Whether iBoot maps the window there is not |
| FA-009 | [T] | The Ghidra project file is empty and no function export exists | Ghidra project file and repository index | Index presence only. No function counts or cross-references were computed |

## Provenance

| Role | File | Bytes | SHA-256 |
|---|---|---|---|
| build manifest | BuildManifest.plist | 411742 | c023bfa8dff4b299aa140699817e53f44fac4ad07cc3a19791d2c39a858021e1 |
| restore archive | iPhone16,2_27.0.1_24A446_Restore.ipsw | 12359719732 | 331bc7088ec5ebfee22b643b878dd0704a13e1a6fd49b376e445865b2fdfab08 |

- External step: IM4P payloads were decompressed earlier with an external IM4P tool; the decompressed files are hashed here, not re-derived
- External step: The device tree payload was converted to JSON earlier with an external tool; this run parses the binary payload directly

Tags: [T] read from a target artifact, [M] stated by the manifest, [I] inferred, [U] unknown.
