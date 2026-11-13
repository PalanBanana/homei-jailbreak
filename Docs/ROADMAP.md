# Roadmap

Nothing here is a promise. Items that depend on security research results are gated on evidence, not on a date.

## 0.1.0 (this release): product foundation

Done: core logic, data schemas, safety gates, catalog and bootstrap simulation, SwiftUI sources, localisation, tests, CI.

## Next, safe engineering

- Compile and test the Swift package and the app on a Mac with Xcode, and record the result.
- Generate PNG icon sizes from the SVG source.
- Add snapshot and accessibility tests for the interface.
- Run `cargo audit` or `cargo deny` in CI.
- Refresh `Data/` from the research tracker through a reviewed, reproducible script.
- Add more fixtures (other product types) as the tracker establishes their SoC mapping.
- Per-screen Korean translation of research text, with attribution to the English source.

## Gated on external research (not started)

A working jailbreak for the research target would first need public, verified results for the individual capabilities that are separate today: sandbox escape, kernel read and write, SPTM compatibility, AMFI compatibility, bootstrap and persistence. It would also need an independent reproduction on the exact device and build. Until those exist in the research tracker, HOMEI stays research-only. Any such milestone would be a separate project decision with its own safety review, and the existing rules against shipping exploit code in this repository would need to be revisited by the owner explicitly.
