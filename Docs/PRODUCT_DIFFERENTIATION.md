# Product differentiation

HOMEI is an independent product. This page separates what is original to HOMEI from ideas that are common across the industry, so neither is overstated.

## Original to HOMEI

| Capability | What is original |
|---|---|
| Evidence-driven compatibility engine | A deterministic matcher over exact product type, version, build and architecture. A positive declaration is automatically reduced to NOT ESTABLISHED unless the evidence it cites contains an exact-scope target result (and, for SUPPORTED, an independent reproduction). Low-confidence, contradicted and wrong-class evidence is ignored. |
| Evidence classes kept separate | Nine classes (device metadata, public statement, Apple documentation, static observation, reported runtime result, independent reproduction, target-compatible result, unverified, unknown) that are never merged. |
| Five-valued outcome | SUPPORTED, UNSUPPORTED, PARTIALLY SUPPORTED, NOT ESTABLISHED, UNKNOWN, with the reasoning shown to the user. |
| Closed-allow-list Safety Manager | Only six read-only in-app operations pass. Everything else, including unknown names, is denied with a reason. There is no override and no hidden path. |
| Privileged capability status | Twelve named capabilities, all explicitly unavailable, each returning an unsupported result. |
| Profile provenance | Every profile is labelled as a manual profile, test fixture, imported authorised observation or public firmware metadata. No label claims a live connection. |
| Identifier-proof profile model | Unknown keys are rejected and values are shape-checked, so identifiers cannot be imported. |
| Offline research index tied to a tracker | Findings point at claim IDs and a pinned tracker commit. Absence of a CVE entry is a recorded limit, not a verdict. |
| Source-hygiene tests | Tests fail the build if product code gains network, process, device or filesystem-write constructs, or an unexpected dependency. |
| Original identity and design | Name, mark, colour tokens and layout were created for HOMEI. |

## Common industry patterns (not claimed as original)

- Splitting a jailbreak-style app into UI, logic and support components.
- Shipping an IPA, and a TrollStore-style TIPA, as release artefacts.
- Large-scale localisation with per-language resource folders.
- A package catalog showing identifiers, versions, descriptions and sources.
- Dependency ordering for a component list.
- A compatibility table keyed by iOS version range.
- Release notes per version.
- A tab-based mobile interface.

## What HOMEI does not have

An exploit, a kernel primitive, a bootstrap that installs, a package manager that installs, device connection code, or any claim of support for the research target. Those would need research results that do not exist yet. See `Docs/ROADMAP.md`.
