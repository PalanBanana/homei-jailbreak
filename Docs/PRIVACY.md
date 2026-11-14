# Privacy

## Never published

IPSW files, kernelcache, SPTM or TXM binaries, Ghidra database files, local absolute paths, personal device identifiers, VM memory dumps, private screenshots, guest disks, signing credentials and API tokens. `.gitignore` and the validator enforce this, and the lab state has no path field.

## May be published

Target manifests, sanitized metadata (names, sizes, hashes), public source links, analysis summaries, test fixtures and reproducible configurations.

## The lab and personal devices

The lab never connects to a physical device: no USB, no pairing, no discovery, no polling. The one narrow, separately authorised read-only identification in the tracker's rules is finished and is not part of this project's operation. The research target is a public profile, and nothing here describes any person's device.

## The host

The exported state says the host is "Linux", its architecture, the QEMU and Ghidra versions and coarse memory figures, so a blocker such as "not enough free memory" can be understood. It does not name a distribution, a machine, a user or a path.

## Checks

The validator rejects UDID-shaped values, 40-character hexadecimal identifiers, email addresses, personal paths, private keys, tokens and device-state phrases in any text file. The Rust test crate scans the exported state for path markers and identifier words, and `LabState::validate` and its Swift mirror refuse a snapshot that contains a path marker.
