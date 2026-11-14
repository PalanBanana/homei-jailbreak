# Guest package manager

A real package manager for the **guest overlay only**. It is not an iOS package manager, and an installation here is not an installation on any iPhone. The existing metadata catalog (`Catalog/`) is separate and remains non-installable.

## Format

An `HPK1` archive: the magic line `HPK1`, a JSON header (at most 1 MiB) and the file payloads concatenated. The header names the package id, version, description, licence, source, dependencies, conflicts and, for every file, its path, size, SHA-256 and executable flag. Limits: 10,000 files, 64 MiB per file, 256 MiB in total. Unknown header fields are rejected. A repository is a directory with an `index.json`: each entry carries the archive name, its SHA-256 and an optional Ed25519 signature.

## Safety properties

| Property | How |
|---|---|
| Explicit approval | Every install, removal and rollback needs an `Approval` value. The command line needs `--approve`. |
| No arbitrary scripts | Maintainer script names (`preinst`, `postinst`, `prerm`, `postrm`, `install.sh` and similar) are rejected, and nothing in a package is ever executed. A header that declares scripts is rejected. |
| No host writes | Writes go through the path guard: relative plain paths only, no `..`, no NUL, no backslash, no colon, no symbolic link on the way. |
| No immutable firmware | The base image and any registered firmware are outside the overlay and cannot be reached. |
| Integrity | The archive is checked against the hash in the index, then each file against the hash in the header. A checksum detects change. It does not prove authenticity. |
| Authenticity | An Ed25519 signature over `HOMEI-PKG-V1`, id, version and archive hash is checked against an explicitly supplied trust root. An unsigned package is refused unless `--allow-unsigned` is given, and is then marked as unsigned. A signature by an unknown key is refused. |
| Transactions | An install or removal is journaled before it changes anything. A failure or an interruption rolls back. `rollback` reverts the last committed transaction. |
| Dependencies | Versions compare numerically (`1.10` is above `1.9`, and `1.0` equals `1.0.0`). Requirements, conflicts and cycles are resolved before anything is written, and dependencies install first. A package that another one needs cannot be removed. |

## Demonstration packages

`homei-sample-lib` and `homei-sample-tool` are inert text fixtures. They are signed with a **public, test-only** key, so the signature shows the mechanism works and proves nothing about a real publisher. They are not research tools.

## Current state

Install, removal, rollback, interruption recovery, tampering, a bad signature, an untrusted key, an unsafe path, a script, a conflict, a cycle and a missing dependency each have a test (20 package tests in total).
