# VM-only research bootstrap

This is a research bootstrap for the isolated guest overlay. **It is not an iOS jailbreak bootstrap.** It never touches a physical device, the host, the original firmware or the base image.

## What it does

`homei-lab bootstrap <workspace>` creates, inside the disposable overlay only:

- a directory layout below `opt/homei-lab`: `bin`, `packages`, `profiles`, `analysis`, `logs`, `manifests`, `workspace`
- `profiles/default.json`, which states that the scope is an isolated research guest and that guest root is not physical device root
- `manifests/bootstrap.json`, with the bootstrap version (`1`), the scope, the layout, the profiles and a state
- `logs/bootstrap.log`
- optionally, a package step through the guest package manager (see [VIRTUAL_PACKAGE_MANAGER.md](VIRTUAL_PACKAGE_MANAGER.md)), which needs `--approve`

## Properties, each covered by a test

- **Idempotent.** A second run reports `Unchanged`, creates nothing and leaves every file except the log identical. A repeat run without a package step keeps the package request an earlier run recorded.
- **Guest-only.** Every write goes through the path guard, so nothing is written outside the overlay.
- **Honest on failure.** A failed package step leaves a `layout_only` state and no package behind. An interrupted package transaction rolls back.
- **Reversible.** Discarding the overlay (with `--confirm`) returns to the base state. The base image is never opened for writing.
- **Versioned and logged.**

## What it is not

It installs no jailbreak, no persistence mechanism and nothing on a device. It has no privileged helper. It runs nothing from a package.
