# Darwin VM backend adapter

`Lab/src/backend.rs` adapts the public `jprx/darwin-vm` project and its `qemu-sptm` QEMU fork. HOMEI does not vendor or modify either one. It pins the commits that were inspected (`edfc19433da8` and `bad6336fb242`), detects whether the pieces are present, builds a launch plan, and classifies what a boot attempt actually showed. Reading a pin does not make upstream trusted.

## What upstream is

A generic `-M darwin` machine driven by a device tree, with QEMU's generic `max` CPU, optional SPTM and TXM monitor images, and a restore ramdisk. There is no per-device machine and no Apple core model. Its README states there is no screen, Wi-Fi, Bluetooth, graphics, GUI apps or SpringBoard. It is therefore a way to run a given kernel on a generic machine, not an iPhone emulator.

## Required inputs

`bootkc` (kernelcache), `dtree` (device tree), `ramdisk.dmg`, `ramdisk.tc`, `sptm` and `txm` for SPTM kernels. Preparing them is a macOS-only upstream script that uses `hdiutil`, `codesign` and `sudo`. HOMEI does not run it and does not run `sudo`. On the checking host the ramdisk cannot be produced.

A tool limitation worth knowing: the unmodified preparation script looks for the monitor image by the chip name from the `ipsw` database (`t8130`). The 24A446 package contains `sptm.t8122.release.im4p` and no `sptm.t8130` entry. The script would silently continue without SPTM or TXM. Selecting the monitor must follow the token the manifest names, and the choice must be recorded (tracker claims C-159 and C-160).

## Preflight

`homei-lab preflight` runs only `qemu-system-aarch64 --version`, `-accel help` and `-M help`, checks which firmware files exist, and compares available memory with the 8 GiB the guest script requests. It reports `can_launch` and a list of blockers. With `--hash` it also hashes the firmware files. It starts no guest.

## Launch

`plan` prints the command and exits. `launch` refuses to run without both `--start-vm` and a passing preflight. The command is `qemu-system-aarch64 -M darwin -bootkc … -dtree … -tc … -ramdisk … -sptm … -txm … -args "<boot-args>" -nographic -serial mon:stdio -m <N>G`. No network option is ever added. Only the programs `qemu-system-aarch64` and `qemu-img` can be started, matched by file name, never through a shell, with a timeout and capped output. `ProcessRunner` has a real implementation and a labelled mock, and a mock result can never count as a real boot.

## Classification order

The status is only as strong as the weakest unmet condition above it:

1. no backend configured: `REFERENCE_DEVICE_ONLY`
2. the backend rejects the target: `UNSUPPORTED_TARGET`
3. QEMU or the `darwin` machine is missing: `BLOCKED_BY_MISSING_QEMU_FEATURE`
4. firmware inputs missing: `BLOCKED_BY_MISSING_FIRMWARE`
5. no boot attempted: `NOT_TESTED`
6. timeout, panic, interruption, process failure or incomplete output: `BOOT_FAILURE`
7. booted but the kernel hash, product, board and chip identifier do not all match the target: `PARTIAL_DARWIN_GUEST`
8. identity matches: `TARGET_KERNEL_BOOTED_HARDWARE_UNVERIFIED`, unless the backend models the target CPU itself **and** a shell was reached, in which case `EXACT_TARGET_VERIFIED`

With `-M darwin` and the generic CPU, step 8 can only end in `TARGET_KERNEL_BOOTED_HARDWARE_UNVERIFIED`.

## Corellium

Recorded as an optional backend with status `NOT_TESTED` and no integration. Public release notes list iOS 27.0.1 as an OS version, but exact-model support for the iPhone 15 Pro Max was not documented in what was read. HOMEI creates no account, accepts no terms, buys nothing and sends no private firmware.
