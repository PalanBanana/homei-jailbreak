# Experiments

Every run in the lab is a record: what was targeted, which backend was used, what the configuration was, what was observed and what is **not** known. Records are stored in the lab workspace and exported, sanitized, in the lab state.

## Record

`experiment_id`, `kind`, the target (`product_type`, SoC, board, build), the backend (`name`, `kind`, versions), the guest OS build **only if established by observation**, firmware artifact hashes, a configuration hash, an optional snapshot id, inputs, a UTC timestamp, the status, the boot outcome, a result sentence, log references (name and hash), limitations and a `reproducible` flag that stays unset until a reproduction is recorded.

`backend.kind` is `none` (a host-side operation, a preflight or static analysis), `mock` (a test double) or `real`. A mock is never a boot.

## State machine

`PLANNED`, `RUNNING`, then exactly one terminal state: `COMPLETED`, `FAILED`, `INTERRUPTED`, `INCONCLUSIVE` or `NOT_REPRODUCIBLE`. A record still marked `RUNNING` when the store is opened was interrupted and is recorded as `INTERRUPTED`, never as a success. Only `COMPLETED` counts as a success. Logs are capped at 1 MiB and referenced by hash.

## Comparison

Two records are comparable only if they describe the same product type, SoC and board. A result from an A18 guest stays an A18 result and is rejected, not folded in. Comparing a record with itself is refused. The comparison reports whether build, backend, configuration, firmware and status are the same, and says when a mock backend was involved.

## Correlation with static analysis

A correlation links an analysis project and its firmware hash to an experiment, and keeps the build, the guest identity and the configuration hash. It is refused when the builds differ, when the experiment did not complete, when the backend was a mock, and when a guest observation is presented as reproduced behaviour of physical hardware. Evidence is classified as a static observation, a decompiled interpretation, an emulator observation, reproduced behaviour, a hypothesis or unknown, and these are never merged.

## Recorded so far

| Id | Kind | Backend | Status | What it shows |
|---|---|---|---|---|
| `exp-0001` | feasibility preflight | none (no VM) | COMPLETED | The host has no `darwin` machine and no firmware inputs. It started nothing. |
| `exp-0002` | guest workspace lifecycle | none (no VM) | COMPLETED | Bootstrap, two signed sample packages, a removal, a snapshot and a verified restore in the overlay. |

No correlation exists, because no real guest was observed. `reproducible` is unset on both.
