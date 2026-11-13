# Device compatibility

## What a status means

| Status | Meaning in HOMEI |
|---|---|
| SUPPORTED | Exact-device, exact-build evidence of a target-compatible result **and** an independent reproduction are on record. |
| PARTIALLY SUPPORTED | At least one of those is on record for part of the capability set. |
| UNSUPPORTED | A record states the combination does not work. |
| NOT ESTABLISHED | Records exist for the device but not enough evidence for this build. Not a statement that it fails. |
| UNKNOWN | HOMEI has no record, no sourced chip mapping, or the profile was rejected. |

A version-only or chip-only match never produces a positive status. A result on another device, another version or another chip never transfers.

## Current state of the research target

| Field | Value |
|---|---|
| Product type | iPhone16,2 (iPhone 15 Pro Max) |
| SoC | A17 Pro |
| iOS version / build | 27.0.1 / 24A446 |
| Architecture | arm64e |
| **Status** | **NOT ESTABLISHED** |
| Readiness | LEVEL 0 / NO-GO |

Why: the cited evidence is an identifier mapping (partially verified), and statements that no complete exploit chain and no installable tool is verified for this device and build. A reported demonstration used a different device and chip. Apple's index lists no CVE entries for 27.0.1, which is recorded as a limit and not as proof of anything. The research tracker holds the full record.

HOMEI shows this state for the target, and it must continue to do so until the evidence classes above exist. The tests and the validator enforce it.

## Test fixtures

| ID | Profile | Result |
|---|---|---|
| FIX-A | iPhone16,2, A17 Pro, 27.0.1, 24A446, arm64e | NOT ESTABLISHED |
| FIX-B | iPhone18,1, A19 Pro, 27.0.1, 24A446 | NOT ESTABLISHED |
| FIX-C | iPhone17,2, A18 Pro, 27.2, 24B119 | NOT ESTABLISHED |

Fixtures are synthetic. They are not connected devices. They carry the `test_fixture` provenance and no identifiers. Fixture A takes the public research-target profile. Its architecture is the arm64e subtype recorded for the 24A446 firmware images (tracker claim C-096). Hardware model and device class are left empty on purpose.

## Separate capabilities

These are tracked as separate questions, each currently unavailable: sandbox escape, container access, cross-process access, kernel read, kernel write, privilege escalation, SPTM compatibility, SPTM bypass, AMFI compatibility, bootstrap, persistence, complete jailbreak. A result for one does not establish another. An application-level boundary result does not establish kernel privilege, a kernel primitive does not establish an SPTM bypass, and a demonstration on one chip does not establish another.

## Signing status

Signing is a dated third-party observation, not a verdict. See `Docs/RECOVERY.md`.
