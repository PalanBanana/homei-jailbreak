# Recovery and data protection

This page explains limits. HOMEI automates no restore, recovery or device operation, and nothing here promises that a device can be recovered.

## Plain facts

- **Firmware signing.** Apple decides which builds can be installed, and it can change that without notice. A status shown by HOMEI is a dated third-party observation, not an Apple statement.
- **Restore.** A restore installs a build that Apple is currently signing. If the installed build is no longer signed, it cannot be reinstalled after an erase.
- **Backups.** A backup holds data and settings, not the operating system. A backup made on a newer iOS version generally cannot be restored to an older one. **A backup does not guarantee a way back to an unsigned build.**
- **Data loss.** Erasing or restoring removes data that is not backed up elsewhere.
- **Saved signing tickets (SHSH blobs).** They do not by themselves guarantee a restore. Whether a ticket can be used depends on the device, the build and tooling HOMEI does not evaluate.
- **App compatibility.** Apps, banking and payment features and enterprise management may react to a modified system. HOMEI has no data about any specific app.
- **Recovery uncertainty.** No recovery path has been verified for the research target.

## Signing observations for the research target (build 24A446)

| Observation | Source | Date | Verified |
|---|---|---|---|
| Listed as not signed by a third-party firmware index | ipsw.me device index (tracker source S-014) | 2026-11-12 | No |
| Reported as still signed by a community repository | Community reference repository (tracker source S-026) | 2026-11-02 | No |

The two observations disagree and neither is from Apple, so the status is **unresolved** (tracker claim C-021). The tracker did not check an Apple-hosted signing record.

## GO / NO-GO

In this release the answer is **always NO-GO**, because HOMEI contains no operational backend and no recovery path is verified. The reasons shown are: no operational backend; no verified recovery path; and the current compatibility status when it is not SUPPORTED.

## What this repository does not contain

A maintainer's private device state. The public record describes the research target as a profile only.
