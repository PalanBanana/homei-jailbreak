# Product requirements

## Purpose

An independent, research-stage framework that tells a user, honestly and with evidence, whether a device and firmware are known to be compatible with a jailbreak, what is not known, and what the risks and recovery limits are. It does not jailbreak anything.

## Initial release scope (0.1.0)

Included: device profile model, compatibility engine, evidence database, research index, safety manager, diagnostics metadata, offline package catalog (metadata), bootstrap planning and simulation, recovery documentation, English and Korean interface, tests, validators and CI.

Excluded: any exploit or privileged operation, any device communication, any installation, any download of third-party binaries, any claim of support.

## Module responsibilities

| Module | Responsibility | Must never |
|---|---|---|
| Device Profile | Hold six metadata fields and provenance | Store identifiers, connect to a device, poll |
| Compatibility Engine | Decide a status from exact evidence | Mark a target compatible from another device, version or chip |
| Evidence Database | Store dated, scoped, classified evidence with limits | Merge evidence classes |
| Research Intelligence | Show sanitized public findings | Label anything exploitable, embed private analysis |
| Safety Manager | Allow only read-only in-app operations | Offer an override or a hidden path |
| Diagnostics | Validate bundled data and export a sanitized report | Read the OS, logs, other apps or credentials |
| Package Catalog | Show metadata | Download or install |
| Bootstrap Planning | Order components and print a simulation | Write, install or change anything |
| Recovery | Explain limits and give GO / NO-GO | Automate a restore or promise recoverability |
| Privileged Capability Status | List capabilities as unavailable | Return a success |

## Safety boundaries

See `Docs/SAFETY.md`. In short: no device interaction, no privileged operation, no untrusted artefact, no remote script, nothing destructive.

## Acceptance criteria

- The target fixture evaluates to NOT ESTABLISHED, and no bundled record is positive.
- Every privileged or unknown operation is denied.
- Source-hygiene tests pass.
- The validator, formatter, linter and tests pass in CI.
- No identifier or personal path appears in the repository.
- Documentation states that the Swift app and the iOS binary have not been built in the development environment.
