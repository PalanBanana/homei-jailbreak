# Roadmap

Nothing here is a promise. Items that depend on security research results are gated on evidence, not on a date.

## 0.1.0: product foundation (done)

Core logic, data schemas, safety gates, catalog and bootstrap simulation, SwiftUI sources, localisation, tests, CI.

## 0.2.0: native build and runtime foundation (done on `v0.2.0-native-build`, not released)

The Swift code compiles and is tested on macOS CI, the app builds and runs its UI tests on the iOS Simulator, the icon set is generated, Rust/Swift parity is tested, and the dependency audit runs in CI.

## Next, safe engineering

- Run VoiceOver in the simulator and record reading order and spoken output.
- Test the sizes between default and the largest accessibility size, iPad and landscape, Increase Contrast and Reduce Transparency.
- Identify the six Apple audit findings that have no addressable element.
- Try the Swift 6 language mode.
- Translate recovery and compatibility explanations into Korean.
- Refresh `Data/` from the research tracker through a reviewed, reproducible script.
- Add more fixtures as the tracker establishes their SoC mapping.

## Gated on external research (not started)

A working jailbreak for the research target would first need public, verified results for the individual capabilities that are separate today: sandbox escape, kernel read and write, SPTM compatibility, AMFI compatibility, bootstrap and persistence. It would also need an independent reproduction on the exact device and build. Until those exist in the research tracker, HOMEI stays research-only. Any such milestone would be a separate project decision with its own safety review, and the existing rules against shipping exploit code in this repository would need to be revisited by the owner explicitly.
