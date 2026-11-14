# Firmware artifact metadata

The lab publishes **metadata only**: an artifact name, size, SHA-256, a coarse format, the build it belongs to and its role. The files themselves (the IPSW, the kernelcache, the SPTM and TXM payloads, extracted extensions) are never committed.

## Source

`analysis/artifact-hashes.json` from the local analysis tree lists each artifact with its size, hashes and the firmware it came from. `parse_artifact_hashes` rejects a malformed digest, keeps only records for the target build (24A446) and tolerates a record with no relative path, such as the full package. `to_public` removes the path and derives the format and role from the artifact's own description, never from guessing at bytes. An artifact it cannot classify is labelled "not classified".

## Verification

`verify_local` hashes a local file and compares it with the record. It reports a missing file or a mismatch. It never opens a firmware file for writing.

## Checksums and authenticity

A matching SHA-256 shows that a file is the one that was recorded and has not changed. It does **not** prove that the file is authentic. Authenticity would need a trusted signature check against Apple, which this lab does not claim to perform.

## Current export

14 artifacts for 24A446: the build manifest, the raw kernelcache container, the decompressed kernelcache payload and its fileset form, eight extracted extensions and the full package. The build manifest (SHA-256 `c023bfa8…`) is the primary source of the target's identity fields (tracker source S-043).

## Not included

The device tree and the restore ramdisk are not present locally, so the backend reports the firmware inputs as missing. See [DARWIN_VM_BACKEND.md](DARWIN_VM_BACKEND.md).
