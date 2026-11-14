# Virtual research lab (v0.3.0)

The lab is a host-side environment for studying the exact research target, an iPhone 15 Pro Max (`iPhone16,2`, Apple A17 Pro) on iOS 27.0.1 (build 24A446), **without touching a physical device**. It is made of a Rust host controller (`homei-lab`), portable data types in `homei-core`, and read-only screens in the iOS app.

## Short answers

| Question | Answer |
|---|---|
| Is this a complete iPhone emulator? | **No.** Upstream `darwin-vm` itself states there is no screen, Wi-Fi, Bluetooth, graphics, GUI apps or SpringBoard. HOMEI adds none of those. |
| Is A17 Pro hardware supported? | **Not established.** `darwin-vm` has one generic machine with QEMU's generic `max` CPU and no Apple core model. It does not list `iPhone16,2` as tested, and it does not reject it either. |
| Can build 24A446 boot? | **Not shown.** Nothing was booted. On the checking host the installed QEMU has no `darwin` machine, and the firmware preparation needs macOS. See [EXACT_TARGET_FEASIBILITY.md](EXACT_TARGET_FEASIBILITY.md). |
| Is there a GUI? | The iOS app shows the lab's exported state. It cannot start, stop or snapshot a guest. A guest has no GUI. |
| Wi-Fi and Bluetooth in the guest? | `UNSUPPORTED`, as stated upstream. |
| Can research tools be put into the guest? | Yes, into the isolated overlay only, through the guest package manager and the VM-only bootstrap. Nothing is installed on a device. |
| Is guest root the same as physical device root? | **No.** `GUEST_ROOT` and `PHYSICAL_DEVICE_ROOT` are distinct types in code. Guest root never implies the other. |
| Which results were reproduced? | **None is marked reproduced.** Two host-side experiments were recorded with `reproducible` unset. No VM result exists. |

The current status is `BLOCKED_BY_MISSING_QEMU_FEATURE` for the checking host, hardware fidelity `UNVERIFIED`. Even the best case with today's public tooling is `TARGET_KERNEL_BOOTED_HARDWARE_UNVERIFIED`, never `EXACT_TARGET_VERIFIED`. A generic ARM64 Linux VM, or another iPhone model, is never substituted and labelled as the target.

## Parts

| Part | Where | Role |
|---|---|---|
| Target manifest | `Research/Targets/iPhone16,2-iOS27.0.1.json`, `Core/src/target.rs` | The fixed identity, with provenance for every field. Rejects any other model, chip, board or build. |
| Fidelity engine | `Core/src/fidelity.rs` | Classifies the VM status from evidence and reports firmware, hardware and function separately. See [VM_FIDELITY.md](VM_FIDELITY.md). |
| Backend adapter | `Lab/src/backend.rs` | Preflight, dry-run launch plan, guarded launch. See [DARWIN_VM_BACKEND.md](DARWIN_VM_BACKEND.md). |
| Guest storage | `Lab/src/storage.rs`, `Lab/src/guard.rs` | Registered base image, disposable overlay, checksummed snapshots. |
| Firmware metadata | `Lab/src/firmware.rs` | Names, sizes and hashes only. See [FIRMWARE_MANIFEST.md](FIRMWARE_MANIFEST.md). |
| Ghidra summary | `Lab/src/ghidra.rs` | Read-only summary of an existing project. See [GHIDRA_INTEGRATION.md](GHIDRA_INTEGRATION.md). |
| Bootstrap | `Lab/src/bootstrap.rs` | See [VIRTUAL_BOOTSTRAP.md](VIRTUAL_BOOTSTRAP.md). |
| Package manager | `Lab/src/packages.rs` | See [VIRTUAL_PACKAGE_MANAGER.md](VIRTUAL_PACKAGE_MANAGER.md). |
| Experiments | `Lab/src/experiments.rs`, `Core/src/experiment.rs` | See [EXPERIMENTS.md](EXPERIMENTS.md). |
| State export | `Lab/src/state.rs`, `Core/src/labstate.rs` | The sanitized `Data/lab/lab-state.json` that the app displays. |
| App screens | `App/Views/LabViews.swift`, `App/Sources/Kit/Lab.swift` | Ten read-only screens in English and Korean. |

## Using the host controller

Every command is explicit. Nothing starts a guest by itself.

```
cargo run -p homei-lab -- target --repo-root .          # validate the target manifest
cargo run -p homei-lab -- preflight                      # read the host environment, start nothing
cargo run -p homei-lab -- plan --firmware <firmware-dir> # print the launch command, start nothing
cargo run -p homei-lab -- launch --firmware <dir> --start-vm   # refused unless preflight passes
cargo run -p homei-lab -- workspace init|usage|snapshot|restore <dir> ...
cargo run -p homei-lab -- demo --workspace <dir> --repo-root .
cargo run -p homei-lab -- export-state --repo-root . --lab-dir <analysis-dir> --out Data/lab/lab-state.json
```

`workspace delete-snapshot` and `workspace discard` do nothing without `--confirm`. `pkg install`, `pkg remove`, `pkg rollback` and `bootstrap` with a package do nothing without `--approve`.

## What the lab does not do

No physical device is discovered, paired, polled or modified. There is no USB passthrough, no host folder mount, no automatic network, no SSH and no background service. `sudo` is not used and is not an allowed program. No exploit, kernel primitive, sandbox escape, SPTM/TXM/AMFI bypass, persistence or device filesystem write exists anywhere in the project. A function or symbol name in the Ghidra summary is never turned into a vulnerability claim.

## Evidence that was recorded

`exp-0001` runs the preflight and records that no `darwin` machine and no firmware inputs are present. `exp-0002` runs the guest bootstrap, installs two inert sample packages in dependency order, removes one, takes a snapshot, restores it and verifies the installed files. Both have `backend.kind = none` and say so: no virtual machine was started.
