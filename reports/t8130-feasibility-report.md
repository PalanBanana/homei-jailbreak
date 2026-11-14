# T8130 feasibility report

Target: iPhone16,2 / D84AP / T8130 / iOS 27.0.1 (24A446). Generated 2026-11-14 from the analysis of 2026-11-14.

**Exact target VM: NOT_ESTABLISHED.** No exact target VM is established. The target has not booted in any backend, and the blockers are listed in the components above

Statuses: VERIFIED, PARTIAL, BLOCKED, UNSUPPORTED, UNVERIFIED. Confidence tags: [T] target artifact, [M] manifest, [I] inferred, [U] unknown. Nothing here is inferred from another SoC.

| Component | Status | Question |
|---|---|---|
| CPU T8130 | UNSUPPORTED | Can the backend model the T8130 CPU (arm64e, GXF/SPRR)? |
| Memory map | PARTIAL | Is the target's physical memory layout modelled? |
| Interrupt controller (AIC, aic,3) | BLOCKED | Can an AIC model for aic,3 be bound to the target's interrupt tree? |
| Timer (aic-timebase) | UNVERIFIED | Is the always-on timer modelled? |
| Power manager (PMGR, pmgr1,t8130) | BLOCKED | Can the PMGR be modelled, and its power domains bound? |
| Display controller (DCP) | UNSUPPORTED | Can the display coprocessor be modelled, and does the backend show a target framebuffer? |
| GPU (AGX) | UNSUPPORTED | Can the GPU firmware and device be modelled? |
| USB controllers | UNVERIFIED | Are the target USB controllers identified and modelled? |
| Storage (ANS, system volume) | BLOCKED | Can the storage coprocessor run, and is a full system volume available? |
| SEP interface | BLOCKED | Can the Secure Enclave firmware be run or emulated? |
| SPTM | BLOCKED | Can the Secure Page Table Monitor run under the backend? |
| TXM | BLOCKED | Can the Trusted Execution Monitor run under the backend? |
| DeviceTree compatibility | PARTIAL | Can the target device tree be loaded by the backend? |
| Kernel compatibility | BLOCKED | Can the target kernel boot on the target platform model? |
| Root filesystem availability | PARTIAL | Is a root filesystem available for the target? |
| GUI feasibility | BLOCKED | Can the target display an interface in the backend? |

## CPU T8130

**UNSUPPORTED.** Identity is verified. The installed backend has no T8130 CPU model

- [M] BuildManifest identity matches iPhone16,2, D84AP, ApChipID 0x8130, build 24A446, 27.0.1 (source: firmware analysis bundle (t8130-firmware-analysis.json); evidence: static manifest; verified 2026-11-14). Limitation: Identity only. It does not show that any hardware model is available
- [T] The installed QEMU (11.1.3) was probed with a generic virt machine and the max CPU only; no T8130 CPU model, darwin machine or SEP model was present (source: logs/phase9-actual-boot-report.md; evidence: recorded backend probe; verified 2026-11-14). Limitation: Recorded in phase 9. Not re-run in this phase, per the static-only constraint

## Memory map

**PARTIAL.** The AIC window translates through the first /arm-io ranges entry. DRAM and carve-outs are filled in at boot and are not in the static tree

- [T] The first /arm-io ranges entry maps child 0x0 to parent 0x210000000 (source: firmware analysis bundle (t8130-firmware-analysis.json); evidence: static device tree; verified 2026-11-14). Limitation: Read with the declared cell widths of the node
- [I] The AIC base translates to 0x2d1000000 (source: research/hardware/t8130-aic-static-correlation.md; evidence: static device tree arithmetic; verified 2026-11-14). Limitation: The arithmetic is established. That iBoot maps the window there is not

## Interrupt controller (AIC, aic,3)

**BLOCKED.** No model. Register semantics are not known: the kernel reads the offsets from the tree, and aic-iack-offset is named in iBoot but absent from the static tree

- [T] The AIC compatible is aic,3 with 1 interrupt cell(s) (source: firmware analysis bundle (t8130-firmware-analysis.json); evidence: static device tree; verified 2026-11-14). Limitation: One cell is Apple's convention, confirmed by the kernel diagnostic string in the analysis
- [I] The upstream Linux AIC bindings require three or four interrupt cells and do not list aic,3 (source: research/hardware/t8130-aic-static-correlation.md; evidence: public source comparison; verified 2026-11-14). Limitation: Comparison with the public Linux tree, read only. It does not show what Apple's driver does
- [T] The kernel has an AppleInterruptControllerV3 personality matched to aic,3, and the offset property names (source: research/hardware/t8130-aic-static-correlation.md; evidence: static kernelcache strings; verified 2026-11-14). Limitation: Strings only. No function cross-references were computed

## Timer (aic-timebase)

**UNVERIFIED.** A node exists at the translated window. Its register semantics and any backend model are unknown

- [T] The aic-timebase node reg address is 0x2d1180000 (source: firmware analysis bundle (t8130-firmware-analysis.json); evidence: static device tree; verified 2026-11-14). Limitation: Node presence and address only. No timer behaviour was tested

## Power manager (PMGR, pmgr1,t8130)

**BLOCKED.** The compatible is not in the public Linux bindings, the tree has no per-domain nodes, and the blob layout is not documented

- [T] The PMGR compatible is pmgr1,t8130, with 83 register windows and 39 power-domain names (source: firmware analysis bundle (t8130-firmware-analysis.json); evidence: static device tree; verified 2026-11-14). Limitation: Names are ASCII tokens from the blob. No record layout is assumed
- [T] The kernel has an AppleT8130PMGR personality matched to pmgr1,t8130 (source: research/hardware/t8130-aic-static-correlation.md; evidence: static kernelcache strings; verified 2026-11-14). Limitation: Strings only

## Display controller (DCP)

**UNSUPPORTED.** The DCP firmware is present and unencrypted, but there is no DCP model. The only framebuffer is a generic placeholder

- [T] The DCP firmware component is analysed (source: firmware analysis bundle (t8130-firmware-analysis.json); evidence: static IM4P container; verified 2026-11-14). Limitation: Firmware for the real coprocessor. Not executed
- [T] The recorded probe screenshot shows the generic QEMU placeholder, not an iOS display (source: research/display/iphone16,2-display-pipeline.md; evidence: recorded backend probe; verified 2026-11-14). Limitation: Generic machine. Not a target result

## GPU (AGX)

**UNSUPPORTED.** GPU firmware is present and unencrypted. The installed backend has no GPU model for the target

- [T] The GFX firmware component is analysed (source: firmware analysis bundle (t8130-firmware-analysis.json); evidence: static IM4P container; verified 2026-11-14). Limitation: Firmware for the real GPU. Not executed
- [T] The installed QEMU (11.1.3) was probed with a generic virt machine and the max CPU only; no T8130 CPU model, darwin machine or SEP model was present (source: logs/phase9-actual-boot-report.md; evidence: recorded backend probe; verified 2026-11-14). Limitation: Recorded in phase 9. Not re-run in this phase, per the static-only constraint

## USB controllers

**UNVERIFIED.** Not researched in the static analysis. No USB model is recorded

- [U] No USB controller analysis is recorded for the target (source: research/limitations/exact-blockers.md; evidence: recorded research note; verified 2026-11-14). Limitation: Nothing was established

## Storage (ANS, system volume)

**BLOCKED.** The ANS firmware is analysed but has no model, and the system volume image is not available locally

- [T] The ANS firmware component is analysed (source: firmware analysis bundle (t8130-firmware-analysis.json); evidence: static IM4P container; verified 2026-11-14). Limitation: Firmware for the real storage coprocessor. Not executed
- [T] The system volume (OS) component is unavailable locally (source: firmware analysis bundle (t8130-firmware-analysis.json); evidence: archive listing; verified 2026-11-14). Limitation: The archive lists it. There is no local copy, and its key service is external

## SEP interface

**BLOCKED.** The SEP firmware is encrypted with a keybag and its keys are not available. No SEP model exists for this target

- [T] The SEP firmware component is encrypted (keys not available) (source: firmware analysis bundle (t8130-firmware-analysis.json); evidence: static IM4P keybag; verified 2026-11-14). Limitation: The keys are not in the archive. No decryption was attempted
- [T] The installed QEMU (11.1.3) was probed with a generic virt machine and the max CPU only; no T8130 CPU model, darwin machine or SEP model was present (source: logs/phase9-actual-boot-report.md; evidence: recorded backend probe; verified 2026-11-14). Limitation: Recorded in phase 9. Not re-run in this phase, per the static-only constraint

## SPTM

**BLOCKED.** The image is analysed, but the backend lacks most of the implementation-defined registers it uses

- [T] The SPTM component is analysed (source: firmware analysis bundle (t8130-firmware-analysis.json); evidence: static IM4P container; verified 2026-11-14). Limitation: Not executed
- [T] In the A13-based model used by the fork, 98 of 127 SPTM implementation-defined registers are undefined (source: research/hardware/sptm-register-census.md; research/hardware/cpreg-gap.json; evidence: static register census; verified 2026-11-14). Limitation: Census of the static image. Lifetime classes are low confidence (indirect calls unresolved)
- [T] The SPTM image is named for T8122 while the device tree is T8130 (source: firmware analysis bundle (t8130-firmware-analysis.json); evidence: static file name; verified 2026-11-14). Limitation: The naming discrepancy is recorded, not resolved

## TXM

**BLOCKED.** The TXM runs under the SPTM, which is itself blocked

- [T] The TXM component is analysed (source: firmware analysis bundle (t8130-firmware-analysis.json); evidence: static IM4P container; verified 2026-11-14). Limitation: Not executed
- [I] The TXM runs under the SPTM in the handoff order (source: research/boot-chain/iphone16,2-handoff-analysis.md; evidence: static boot-chain analysis; verified 2026-11-14). Limitation: The order is taken from the boot-chain notes. Not observed at run time

## DeviceTree compatibility

**PARTIAL.** The tree parses and its identity is correct. The backend cannot load it, and boot-time values are missing

- [T] The root compatible is D84AP, iPhone16,2, AppleARM (source: firmware analysis bundle (t8130-firmware-analysis.json); evidence: static device tree; verified 2026-11-14). Limitation: Static template from the restore archive
- [T] The tree has 379 nodes (source: firmware analysis bundle (t8130-firmware-analysis.json); evidence: static device tree; verified 2026-11-14). Limitation: Node count of the binary payload

## Kernel compatibility

**BLOCKED.** No boot has been observed. The kernel configuration token names T8122 while the platform is T8130, and no T8130 platform model exists

- [T] The kernel is a arm64e MH_FILESET image with 264 fileset entries (source: firmware analysis bundle (t8130-firmware-analysis.json); evidence: static Mach-O; verified 2026-11-14). Limitation: Header and load commands only
- [T] The kernel build reports xnu-13432.2.10~2 with configuration RELEASE_ARM64_T8122 (source: firmware analysis bundle (t8130-firmware-analysis.json); evidence: static kernel strings; verified 2026-11-14). Limitation: The T8122 token against the T8130 platform is unresolved

## Root filesystem availability

**PARTIAL.** The restore ramdisk is present and unencrypted. The system volume is not available locally

- [T] The restore ramdisk is analysed (source: firmware analysis bundle (t8130-firmware-analysis.json); evidence: static IM4P container; verified 2026-11-14). Limitation: A restore environment, not the installed system
- [T] The system volume (OS) component is unavailable locally (source: firmware analysis bundle (t8130-firmware-analysis.json); evidence: archive listing; verified 2026-11-14). Limitation: Its key service is external. No decryption was attempted

## GUI feasibility

**BLOCKED.** No boot has been observed. The display, the GPU and the kernel are each blocked

- [T] No target boot or display output has been observed (source: logs/phase9-actual-boot-report.md; evidence: recorded probe; verified 2026-11-14). Limitation: The probes used a generic machine and are not target boots

