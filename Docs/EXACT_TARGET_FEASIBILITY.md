# Exact target feasibility: iPhone 15 Pro Max / A17 Pro / iOS 27.0.1 (24A446)

Investigation date: 2026-11-14. Method: public repositories and pages read through the GitHub API and HTTP, the public Apple CDN package directory read with range requests, and read-only checks of the Linux development host. Nothing was booted, no account was created, no terms were accepted, and no physical device was touched.

## Verdict

**An exact A17 Pro / iPhone 15 Pro Max / iOS 27.0.1 (24A446) guest has not been booted, and the evidence says it cannot be booted from this host today.** Recorded status: `BLOCKED_BY_MISSING_QEMU_FEATURE` for this host, with further blockers below. Hardware fidelity: `UNVERIFIED`. Even in the best case with the existing public tooling, the result would be a generic Darwin machine fed this device's firmware, not an A17 Pro hardware model. So the best status the current design could ever reach is `TARGET_KERNEL_BOOTED_HARDWARE_UNVERIFIED`, never `EXACT_TARGET_VERIFIED`.

## 1. Target identity (verified from the primary source)

Read from `BuildManifest.plist` inside Apple's own 24A446 package (tracker source S-043). The file was fetched again on 2026-11-14 and matches the tracker's earlier record: SHA-256 `c023bfa8dff4b299aa140699817e53f44fac4ad07cc3a19791d2c39a858021e1`, 411,742 bytes.

| Field | Value | Where it comes from |
|---|---|---|
| ProductType | `iPhone16,2` | `SupportedProductTypes` in the manifest. Verified |
| Version / build | 27.0.1 / 24A446 | `ProductVersion`, `ProductBuildVersion`. Verified |
| Board (DeviceClass) | `d84ap` (D84AP) | all three build identities. Verified |
| CPID | `0x8130` | `ApChipID`, all three identities. Verified |
| BDID | `0x06` | `ApBoardID`, all three identities. Verified |
| Kernelcache | `kernelcache.release.iphone16` | package listing. Verified |
| Commercial model | iPhone 15 Pro Max | `ipsw` device database and tracker claim C-025. Partially verified |
| SoC name | Apple A17 Pro | tracker claim C-026. Partially verified |
| CPU name `t8130` | `t8130` | `ipsw` device database (platform field) and a community wiki (C-160). **Not an Apple statement.** |
| Architecture | arm64e | arm64e subtype bit in the kernelcache and monitor headers (C-096). Verified |

One discrepancy is recorded rather than smoothed over: the package's monitor image is named `sptm.t8122.release.im4p`, not `t8130` (tracker claim C-159, verified). The chip identifier and the monitor token disagree for the A17 Pro, while for the A18 Pro and A19 Pro they agree.

## 2. What `darwin-vm` is and what it supports

Upstream `jprx/darwin-vm`: MIT licence, default branch head `edfc194` (2026-10-05), created 2026-08-27, no tagged releases. It pins the `qemu-sptm` submodule at `bad6336`. The `qemu-sptm` repository reports its licence as `NOASSERTION` (it is a QEMU fork, and QEMU is GPL-2.0-or-later). HOMEI does not vendor either project.

| Question | Finding | Evidence |
|---|---|---|
| Is it a full iPhone emulator? | **No.** Its own README says there is no screen, Wi-Fi, Bluetooth, graphics, GUI apps or SpringBoard. It boots to a bare root shell. | README, "What this is not" |
| Is the A17 Pro / iPhone16,2 listed as tested? | **No.** Tested rows are `iPhone13,2`, `iPhone14,5`, `iPhone14,7`, `iPhone15,4` (A16), `iPhone17,3` (A18) and `iPhone18,3` (A19). Neither `iPhone16,1` nor `iPhone16,2` appears. | README, "Tested Configurations" |
| Is the target build tested? | **No.** The tested iOS 27.0 row is build 24A437. The target is 27.0.1 / 24A446. | README |
| Do upstream issues mention the A17? | None found for `A17`, `t8130` or `iPhone16`. The only hit for "15 Pro" is an unrelated Apple Watch issue. GitHub search is not exhaustive. | issue search |
| Is there an A17 CPU model? | **No.** The `darwin` machine's default CPU is QEMU's generic `max`. There is no Apple core model. | `qemu-sptm/hw/arm/darwin.c` at the pinned commit |
| Is there a per-device machine? | **No.** One generic machine, `-M darwin`. Device identity comes only from the supplied device tree, kernelcache, SPTM, TXM and ramdisk. | `run.sh`, `darwin.c` |
| Does the code reject unknown SoCs? | No allow-list exists. `get_soc_gen` is marked unused. A device is neither rejected nor confirmed by design. | `darwin.c` |

The A19 tested row is the only one marked as needing different treatment upstream (MIE), and a `FEAT_CPA` gap is documented for one Mac. Success on the A16, A18 or A19 rows says nothing about the A17 Pro, and 24A437 says nothing about 24A446.

## 3. Blockers found for this target and this host

1. **The installed QEMU has no `darwin` machine.** QEMU 11.1.3 lists 111 machine types and none is `darwin`. Its CPU list has generic Arm cores and `max`, and no Apple model. The required fork (`qemu-sptm`) is not built here. `BLOCKED_BY_MISSING_QEMU_FEATURE`.
2. **Firmware preparation needs macOS.** `get_files.sh` patches the ramdisk with `hdiutil`, `codesign` and `sudo chown`. On a non-Mac it stops after the download and prints that it cannot patch the ramdisk. This host is Linux, and HOMEI does not run `sudo`. `BLOCKED_BY_MISSING_FIRMWARE` for a bootable ramdisk.
3. **Required inputs are absent locally.** The kernelcache and monitor payloads are extracted (they feed the existing Ghidra project), but the device tree and the restore ramdisk are not present. The device tree is a 59 kB file in the package, so it could be fetched. The patched ramdisk cannot be produced here (blocker 2).
4. **The unmodified preparation script mis-handles this device.** It looks for `sptm.<chip>.release` using the chip name from `ipsw`, which is `t8130`. The 24A446 package contains `sptm.t8122.release.im4p` and **zero** `sptm.t8130…` entries. The script treats a missing match as "this chip has no SPTM" and continues without SPTM or TXM, which would produce a firmware set that cannot boot an SPTM kernel. This is a tool limitation, not a device fact. A wrapper would have to select the monitor image by the token the manifest actually names, and record that choice.
5. **Memory.** The guest script requests 8 GiB. This host has 30 GiB total but about 5 GiB available at the time of the check.
6. **KVM does not help.** `/dev/kvm` exists, but an x86-64 host cannot hardware-accelerate an arm64 guest, so only QEMU's TCG emulation applies. Boot would be slow.

All six were checked or read directly. None depends on an assumption about the A17 Pro.

## 4. Corellium

Public pages read: the release notes and the iOS device page. No account, trial, purchase or upload.

- Release 7.14.0 (2026-10-08) lists "Added support for iOS/iPadOS 27.0.1". It is an OS-version line with no device list. An earlier release announced iPhone 15, 15 Plus, 15 Pro and 15 Pro Max devices with iOS up to 17.0.3.
- The device page has feature and limitation text and no model matrix. It names Bluetooth, NFC and cellular as unsupported and notes the lack of GPU (Metal) support.
- OS support does not establish hardware support. **Exact-model support for iPhone 15 Pro Max on 27.0.1 is not documented in what was read, so it is `NOT_TESTED` and unestablished.**
- It is a commercial service with a free-trial link. Terms, trial limits and licensing were not evaluated.
- `iPhone15,3` is the iPhone 14 Pro Max (A16). It is not an A17 Pro device.

Corellium would be an optional backend only. HOMEI contains no Corellium integration in this milestone.

## 5. Status table

| Question | Status | Why |
|---|---|---|
| Exact target VM | `BLOCKED_BY_MISSING_QEMU_FEATURE` on this host | no `darwin` machine; further blockers above |
| `darwin-vm` support for A17 Pro / `iPhone16,2` | not tested upstream; not rejected by design | absent from the tested table, no SoC allow-list |
| `darwin-vm` support for 24A446 | not tested upstream | tested build is 24A437 |
| Hardware fidelity | `UNVERIFIED` | generic machine, generic `max` CPU |
| Normal iPhone GUI | `UNSUPPORTED` | upstream states no screen or SpringBoard |
| Wi-Fi and Bluetooth | `UNSUPPORTED` | upstream states none |
| Kernel research in a guest | `NOT_TESTED` | nothing was booted |
| Static analysis of the exact build | done | existing Ghidra project on `kernelcache.release.iPhone16,2` (see `Docs/GHIDRA_INTEGRATION.md`) |

## 6. What would unblock it

On a Mac: run the preparation with the monitor selected by the manifest's token, fix permissions, and produce `firmware/`. On Linux: build `qemu-sptm` at the pinned commit with an unprivileged build directory, and provide at least 8 GiB of free memory. Then boot once, record the serial log, the kernel version string and the device-tree identity, and compare the kernelcache hash with the manifest. Even then the honest result is "this kernel booted on a generic Darwin machine", and `docs/VM_FIDELITY.md` defines how that is worded.

## 7. Reproducing these checks

```
gh api repos/jprx/darwin-vm/readme --jq .content | base64 -d     # tested table, limits
gh api repos/jprx/qemu-sptm/contents/hw/arm/darwin.c?ref=bad6336fb242
ipsw device-info -j -d iPhone16,2                                  # board and platform from the local database
ipsw info --remote <S-043 URL> --list                              # package listing (zip directory only)
ipsw extract --remote <S-043 URL> --pattern BuildManifest.plist    # ApChipID, ApBoardID, DeviceClass
qemu-system-aarch64 -M help | grep -i darwin                       # no match on this host
```
