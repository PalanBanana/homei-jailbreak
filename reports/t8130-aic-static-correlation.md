# T8130 (iPhone16,2 / D84AP, 24A446): AIC static correlation (Phase 11.1)

Date: 2026-11-14. Static only. Nothing was executed, no SPTM, TXM or iBoot code was run, no QEMU was started, and no firmware was downloaded or decrypted.

Tags: **[T]** = read from a target artifact (DeviceTree, iBoot, kernelcache, existing Ghidra index). **[L]** = read from the public Linux tree (read only, no text copied). **[I]** = inference from [T] and [L]. **[U]** = unknown.

Supersedes the "unresolved" items for AIC in `t8130-aic-pmgr-feasibility.md` §1.3 and §1.2. That note stays as the earlier record. The conclusions below are new evidence, not a rewrite of its history.

## Summary

| Question | Result |
|---|---|
| AIC node location | `/arm-io/aic` [T] |
| Compatible | `aic,3` (single string) [T] |
| `#interrupt-cells` | 1 [T] |
| Candidate base `0xC1000000` | Confirmed as the stored `reg` value of `/arm-io/aic` [T]. Relative to `/arm-io`. |
| Translated base `0x2D1000000` | Established as an arithmetic consequence of the first `/arm-io` `ranges` entry, which contains the AIC window [T for the entry, I for the translation]. |
| One-cell property: Apple-specific or parse error? | **Apple-specific convention, not a parse discrepancy.** The target kernel's own diagnostic string says `#interrupt-cells` for the SoC must be 1 [T]. |
| Does the upstream Linux driver accept it? | No. Upstream AIC2 and AIC3 specs require 3 or 4 cells [L]. The DeviceTree must not be rewritten to fit Linux. |
| Are AIC register offsets hard-coded in the kernel? | Not established as hard-coded. The kernel contains the DT property names that carry them (`aicglbcfg-offset`, `intmaskset-stride`, `extint-baseaddress`, and others) [T]. Their values come from the tree at boot. |
| A literal reference to `0x2D1000000` or `0xC1000000` in code? | **None found.** No PC-relative literal load targets any candidate constant in iBoot, TXM, SPTM or the kernelcache [T]. |
| Other occurrences | An isolated `0x2D1000000` (64-bit) in an iBoot table of physical-address-like values, not identified [I, low confidence]. |

## 1. The AIC node (complete record)

Source: `research/hardware/DeviceTree.d84ap.json` (from `extracted/devicetree/DeviceTree.d84ap.im4p`). Dump script: `research/hardware/aic_dump.py`.

| Property | Value | Note |
|---|---|---|
| Path | `/arm-io/aic` | child of `/arm-io` (`arm-io,t8130`) |
| `compatible` | `"aic,3"` | the only string; no vendor-prefixed fallback |
| `device_type` | `"interrupt-controller"` | |
| `interrupt-controller` | `"master"` | |
| `#address-cells` | 0 | node's own children |
| `#interrupt-cells` | **1** | see §3 |
| `reg` | `{addr: 3238002688 = 0xC1000000, size: 1884160 = 0x1CC000}` | stored relative to `/arm-io` |
| `reg-names` | **absent** | upstream AIC2 requires `reg-names` [L] |
| `AAPL,phandle` | 32 | every CPU's `interrupt-parent` is `[32]` [T] |
| Offset and stride properties | `rev-offset` 0, `cap0-offset` 4, `maxnumirq-offset` 12, `aicglbcfg-offset` 20, `pubstampcfg0-offset` 4096, `pubstamplo0-offset` 4104, `pubstampdhi-offset` 4108, `extint-baseaddress` 65536, strides 18944 (0x4A00) for `extintrcfg`, `hwintmon`, `intmaskclear`, `intmaskset` | [T] |
| `#main-cpus`, `#shared-timestamps` | 6, 16 | [T] |
| `max_irq_interrupt_umask`, `cap0_max_interrupt_umask` | 65535 each | [T] |
| `aic-iack-offset` | **absent from this node** | see §5 |

Parent `/arm-io`: `#address-cells` 2, `#size-cells` 2, `compatible` `arm-io,t8130`, `device_type` `t8130-io`.

Sibling `/arm-io/aic-timebase`: `reg` `{addr: 12097945600 = 0x2D1180000, size: 4096}` [T]. Under the same `ranges` entry, its child address is `0xC1180000` (= `0x2D1180000 − 0x210000000`), which is the same window scheme as the AIC. [I, supports the translation.]

## 2. `ranges` entry parsing

`/arm-io` `ranges` is 144 bytes [T]. Entry width is child address (2 cells) + parent address (2 cells) + size (2 cells) = 48 bytes, so six entries:

| # | Child base | Parent base | Size |
|---|---|---|---|
| 0 | `0x000000000` | `0x210000000` | `0x140000000` |
| 1 | `0x700000000` | `0x700000000` | `0x380000000` |
| 2 | `0x580000000` | `0x580000000` | `0x080000000` |
| 3 | `0x5A0000000` | `0x5A0000000` | `0x020000000` |
| 4 | `0x5C0000000` | `0x5C0000000` | `0x040000000` |
| 5 | `0x600000000` | `0x600000000` | `0x080000000` |

The AIC window `[0xC1000000, 0xC11CC000)` lies inside entry 0 (child range `[0, 0x140000000)`). It translates to `[0x2D1000000, 0x2D11CC000)` [I, arithmetic checked]. No other entry contains it. The translation is therefore **established** as an arithmetic consequence of the tree, given the `ranges` entry is read with the cell widths above. The physical address is confirmed only against the bootloader's own use, which §4 did not show.

## 3. The one-cell property

Upstream [L]:

- `apple,aic.yaml` (AIC v1): `#interrupt-cells` `const: 3`.
- `apple,aic2.yaml` (AIC v2 and v3): `#interrupt-cells` `minimum: 3`, `maximum: 4`. The first cell is the interrupt type. The `apple,t8122-aic3` entry is the upstream AIC3 binding, which is **not** used here to infer T8130 behaviour.
- `irq-apple-aic.c`: rejects a specifier with `param_count < 3` or `> 4`. The driver matches `apple,aic`, `apple,aic2`, `apple,t8122-aic3` and the older entries. It does not match `aic,3`.

Target [T]:

- The AIC has `#interrupt-cells = 1`.
- The kernelcache contains the diagnostic string: `Interrupt count mismatch between EDT and IOService - ensure #interrupt-cells for SoC is set to 1` (two copies, at file offsets `0x6eb0d5` and `0x711d65`). The kernel expects a one-cell SoC interrupt-cell count [T].
- The kernelcache matches the AIC compatible with an IOKit personality. The personality's class is `AppleInterruptControllerV3` and its `IONameMatch` is `aic,3` (file offset `0x40f990e`) [T].
- The kernel's `interrupts` consumers in the tree are mostly single-cell arrays. Of the AIC consumers (`interrupt-parent` 32), 52 have one element. Other arrays have several elements, such as `/arm-io/wdt` `[443, 0]`, `/arm-io/gpio` `[268 … 274]`, and `/arm-io/error-handler`. Whether these are several single-cell IRQs or one multi-cell specifier cannot be decided from the DT alone [U].

Verdict: **Apple-specific convention**. The target's own kernel documents and enforces the one-cell form for the SoC. This is not a parser error in `dt_analyze.py`, which reads `#interrupt-cells` directly from the node. The Apple DeviceTree is not rewritten to match Linux. A Linux irqchip cannot bind this tree unchanged [I, consistent with the earlier record].

## 4. Literal and reference search

Method: `research/hardware/ldr_xref.py`, output `research/hardware/work/aic-literal-xref.json`.

Two tests:

1. **Byte match** for the candidate values as stored constants: `u32 0xC1000000`, `u64 0x2D1000000`, and the low word `u32 0xD1000000`.
2. **Literal-pool xref**: every PC-relative `LDR`/`LDRSW` (literal) instruction was decoded and its target compared with each match. A hit only counts as a reference if a literal load points at it.

| Payload (decompressed) | `u32 0xC1000000` raw / aligned / literal loads | `u64 0x2D1000000` raw / aligned / literal loads | Notes |
|---|---|---|---|
| iBoot (`iBoot.d84.RELEASE.bin`, 3.07 MB) | 28 / 0 / 0 | 2 / 2 / 0 | two aligned 64-bit hits, see below |
| TXM (557 KB) | 2 / 0 / 0 | 0 | |
| SPTM (`sptm.t8122.release.bin`, 1.21 MB) | 0 | 0 | |
| kernelcache (73 MB) | 1238 / 2 / 0 | 0 | 2 aligned hits sit in float or small-integer tables |

Interpretation:

- **No literal load references any candidate constant** in any payload [T]. The byte matches are noise: unaligned bytes inside instruction words or other values.
- **iBoot `0x2D1000000` at file offsets `0x33E08` and `0x18A0E8`**: the first sits in a run of 64-bit values such as `0x1FCF00000`, `0x200000000`, `0x31E000000`, `0x1FC3B4000`, `0x1FC3B7000`, which look like MMIO page-aligned addresses. The second has zero-filled neighbours. Neither owner is identified [U, low confidence]. This is **not** evidence that iBoot programs the AIC. It is a candidate table entry only.
- **Kernelcache aligned `0xC1000000` hits** (`0x6A4790`, `0x4520ABC`) sit among float constants and small 32-bit values. They are not AIC references [I].

Why the search finds no base constant: the kernel gets the AIC geometry from the DT. Its strings contain the property names `aicglbcfg-offset`, `cap0-offset`, `maxnumirq-offset`, `pubstampcfg0-offset`, `intmaskset-stride`, `extint-baseaddress`, and `AAPL,interrupts` [T]. The register layout is therefore read at boot, not compiled in [I]. Only the boot-time DT carries the base address, so a literal search cannot find it.

### Base address reference versus register-offset access

- A **base reference** would be a literal or a `reg` value (found: `reg` = `0xC1000000`, in the DT, not in code).
- A **register-offset access** would be a load or store at `base + offset` (for example `base + 0x4A00 × n`, or the `pubstamp` offsets 4096/4104/4108). These offsets are small numbers and are too common to search for meaningfully. No access was identified. Register-offset semantics are **[U]**.

### Existing Ghidra cross-references

The existing kernel Ghidra project (kept outside this repository) stores the names `AppleInterruptControllerV3` and `AppleT8130PMGR` in its database index [T]. It does not store the strings `aic,3` or `pmgr1,t8130` in the index. No function cross-references were pulled, because a headless Ghidra run needs several GiB of RAM and the host had about 8 GiB available. Existing `reports/ghidra-*.md` in the tracker do not mention AIC or PMGR. The next Ghidra step is listed in §7.

## 5. Discrepancies that remain

1. **`aic-iack-offset`**: this string appears in iBoot's table of DT property names, next to `aic`, `cpu%d`, `timebase-frequency` and `pmgr` [T]. It is **absent** from the static DeviceTree [T]. Either iBoot reads it only when present, or the static template does not carry the value that the boot-time tree would. Not resolved.
2. **PMGR and AIC are not in the same binding family.** The AIC is `aic,3`. The PMGR is `pmgr1,t8130`. Both are Apple-internal names and neither is in upstream Linux [L].
3. **Interrupt-cell arrays**: see §3. Ambiguous for multi-element arrays [U].
4. **Single versus two regions**: upstream AIC2 names two regions (`core` and `event`). The target gives one `reg`. The register partition is **[U]**.
5. **SPTM payload name**: the SPTM image is named `sptm.t8122.release`. This is a T8122 name on a T8130 build, the same ambiguity already recorded in the earlier analysis (tracker D-001). It is not used here to infer AIC behaviour.

## 6. Conclusions for the T8130 model

- The AIC base is `0x2D1000000` [I, established arithmetically from the tree's `ranges`], size `0x1CC000`.
- The one-cell specifier is Apple's convention for this SoC, confirmed by the kernel's own diagnostic [T]. A HOMEI model must read interrupt specifiers with one cell, not three.
- The AIC register layout is **not** fixed by any compiled constant in the payloads searched. It comes from the DT at boot. A model of AIC registers therefore needs the boot-time DeviceTree or the register semantics from a dynamic source, which this phase does not have.
- No T8130 AIC model is implemented here. Nothing here supports an exact A17 Pro VM.

## 7. Next evidence step (concrete)

1. Search the **kernelcache** for the AppleInterruptControllerV3 method that reads `aicglbcfg-offset` and `intmaskset-stride` (Ghidra, function-level xrefs to the property-name strings). This gives the register-offset semantics [T] if the xref resolves.
2. Search **iBoot** for the code that reads `aic-iack-offset` (cross-reference to the string address at `0x1CE2D5`), to see whether it is a DT property read or a fixed offset.
3. Keep the physical base `0x2D1000000` as **inferred** until one of those reads confirms the arithmetic.

## Limitations

- Static only. No boot, no emulation, no register read.
- `DeviceTree.d84ap.json` is a static IPSW template. Values filled in by iBoot at boot, such as DRAM and carve-outs, are not in it.
- The literal-pool search covers direct PC-relative loads only. Indirect or computed addresses (ADRP plus ADD or LDR offset, built from a page base) were not searched. An ADRP pair cannot be resolved without the load base, which is unknown [U].
- Ghidra cross-references were not computed (see §4).

## Reproduction

```
python3 -I research/hardware/aic_dump.py research/hardware/DeviceTree.d84ap.json arm-io/aic arm-io/pmgr
python3 -I research/hardware/ldr_xref.py research/hardware/work/aic-literal-xref.json
```

The second command reads the decompressed payloads under `extracted/`, which are not published.
