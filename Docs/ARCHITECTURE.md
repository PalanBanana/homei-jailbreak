# Architecture

HOMEI is a set of small, independently testable modules. The portable core is written in Rust and is built and tested on the host. The iOS interface is SwiftUI over a Foundation-only Swift kit that mirrors the core. There is no privileged backend anywhere.

```
HOMEI application (App/, SwiftUI)
 |
 +-- Presentation layer -------- Views, Components, Theme, Localizable.strings (en, ko)
 |       |
 |       v
 +-- HomeiKit (Swift, App/Sources/Kit) ---------- mirror of the Rust core, Foundation only
 |
 +-- Device Profile ------------ six fields + provenance, strict validation
 +-- Compatibility Engine ------ exact match, evidence-gated, five outcomes
 +-- Evidence Database --------- nine evidence classes, scoped and dated
 +-- Research Intelligence ----- sanitized findings pointing at the tracker
 +-- Safety Manager ------------ closed allow-list, explicit denials
 +-- Diagnostics Metadata ------ data validation, sanitized export
 +-- Package Catalog ----------- offline metadata, nothing installable
 +-- Bootstrap Planning -------- dependency order + simulation only
 +-- Recovery Documentation ---- limits, signing observations, GO / NO-GO
 +-- Privileged Capability Status  twelve capabilities, all unavailable
```

## Technology

| Layer | Language | Why |
|---|---|---|
| Portable core, catalog, bootstrap simulation, CLI, tests | Rust (`serde`, `serde_json` only) | Builds and tests on the Linux host. `unsafe` is forbidden. |
| Data validation | Python with `jsonschema` | Checks the JSON data against schemas and adds privacy and consistency checks. |
| iOS interface and shared kit | Swift / SwiftUI | The supported way to build an iOS app. **Not compiled in the Linux environment.** |
| Data | JSON with JSON Schema | Shared by Rust, Swift and Python. |

Three languages exist because a real iOS app needs Swift, a verifiable core needed a toolchain that exists on the host, and data validation is shortest in Python. The Swift kit mirrors the Rust logic by design, and the same rules are tested in both (`Tests/` and `Tests/SwiftTests/`). Keeping the two in step is a maintenance cost, recorded in `Docs/ROADMAP.md`.

## Crates and directories

| Path | Contents |
|---|---|
| `Core/` | `homei-core`: `device_profile`, `evidence`, `compatibility`, `research`, `safety`, `diagnostics`, `recovery`, `data` |
| `Catalog/` | `homei-catalog` plus `Packages/`, `Sources/`, `Schemas/` |
| `Bootstrap/` | `homei-bootstrap` (`models`, `planning`, `simulation`) plus `Models/` data and schema |
| `Tools/homei-cli/` | `homei`: a host command line view of the core |
| `Data/` | `evidence.json`, `compatibility.json`, `research-index.json`, `device-fixtures.json`, `schemas/` |
| `App/` | SwiftUI sources, `Sources/Kit` (HomeiKit), resources, `project.yml` |
| `Tests/` | Rust suites by theme, plus `SwiftTests/` |
| `scripts/` | `validate_data.py` |

The `Bootstrap/Planning` and `Bootstrap/Simulation` directories in the original structure sketch became `planning.rs` and `simulation.rs` under `Bootstrap/src/`, because Cargo expects sources there. The documentation directory is `Docs/`.

## Data structures

- `DeviceProfile`: `product_type`, `product_version`, `build_version`, `device_class?`, `hardware_model?`, `cpu_architecture?`, `provenance`. Unknown keys are rejected.
- `Evidence`: `id`, `class`, `scope` (product type, version, build, SoC), `source_urls`, dates, `confidence`, `limitations`, `contradicting_ids`.
- `Declaration`: a compatibility record for one device and build, with `status` and cited `evidence_ids`.
- `Decision`: `status`, matched record, `downgraded`, profile provenance, explanation lines.
- `Finding`: a sanitized research statement with a status, scopes, evidence and a tracker claim ID. No "exploitable" state exists.

## Compatibility decision rules

1. An invalid profile (bad shape, identifier-like value) gives **UNKNOWN**.
2. A product type with no sourced SoC mapping gives **UNKNOWN**.
3. Look for a record with the same product type, version and build (and the same architecture when both sides state one).
4. No exact record: if records exist for the product type, **NOT ESTABLISHED** (other versions and devices do not transfer). Otherwise **UNKNOWN**.
5. An exact record declaring UNSUPPORTED, NOT ESTABLISHED or UNKNOWN passes through.
6. An exact record declaring SUPPORTED stays so only if its cited evidence has an exact-scope `target_compatible_result` **and** an exact-scope `independent_reproduction`. PARTIALLY SUPPORTED needs at least one of them. Otherwise the state is reduced to **NOT ESTABLISHED** and `downgraded` is set.
7. Evidence counts only if it is cited by the record, has exact scope, is not low confidence and has no contradicting records.

## Safety Manager

A closed allow-list of six read-only in-app operations. Every other operation, including unknown names, is denied with a reason. External artefacts, remote scripts and unverified compatibility are denied. See `Docs/SAFETY.md`.

## Dependency direction

`Core` depends on nothing in the workspace. `Catalog` and `Bootstrap` depend on `Core`. The CLI and the tests depend on all three. The app depends on HomeiKit only.
