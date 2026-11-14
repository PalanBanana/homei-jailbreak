# VM fidelity

Three questions are answered separately and never merged into one score.

| Axis | Question | Statuses |
|---|---|---|
| A. Firmware | Does the analysis refer to the exact OS build? | `EXACT`, `UNVERIFIED`, `UNSUPPORTED` |
| B. Hardware | Does the virtual machine reproduce the intended hardware target? | `EXACT`, `PARTIAL`, `APPROXIMATE`, `UNSUPPORTED`, `UNVERIFIED` |
| C. Functional | Which features are actually present in the guest? | one status per feature, including `NOT_TESTED` |

The VM status is a separate field with nine values: `EXACT_TARGET_VERIFIED`, `TARGET_KERNEL_BOOTED_HARDWARE_UNVERIFIED`, `PARTIAL_DARWIN_GUEST`, `REFERENCE_DEVICE_ONLY`, `UNSUPPORTED_TARGET`, `BLOCKED_BY_MISSING_FIRMWARE`, `BLOCKED_BY_MISSING_QEMU_FEATURE`, `BOOT_FAILURE` and `NOT_TESTED`. The classification rules are in [DARWIN_VM_BACKEND.md](DARWIN_VM_BACKEND.md).

## Rules

- No positive result comes from a similar CPU generation, a nearby build or a guest that merely produced a shell.
- A guest is "the target kernel" only if the kernel hash matches the reference **and** the device tree product, board and chip identifier match the target.
- `EXACT_TARGET_VERIFIED` also needs a backend that models the target CPU itself. A generic machine with a generic CPU is `APPROXIMATE` at best.
- A feature that upstream states is absent (SpringBoard, graphics, Wi-Fi, Bluetooth) is `UNSUPPORTED` by design, and a feature that was not run is `NOT_TESTED`.
- Guest root is not physical device root. A result in a guest says nothing about a physical device.
- Reference metadata can only carry `NOT_TESTED` or `REFERENCE_DEVICE_ONLY`.

## How a result is worded

| What happened | Wording |
|---|---|
| Nothing booted | "No exact A17 Pro virtual machine has been demonstrated." |
| The target's kernel booted on a generic machine | "This kernel booted on a generic Darwin machine. Hardware fidelity is unverified." |
| Another model's guest booted | "A Darwin guest booted. It is not shown to be the target." |

## Current report

Firmware identity matches the reference build but is `UNVERIFIED` because no guest was observed. Hardware is `UNVERIFIED`. Functional: SpringBoard, graphics, Wi-Fi and Bluetooth `UNSUPPORTED`; guest boot, serial console and kernel research `NOT_TESTED`. VM status `BLOCKED_BY_MISSING_QEMU_FEATURE` on the checking host.
