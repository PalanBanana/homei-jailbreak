# UI and accessibility audit

What was measured on the iOS Simulator, how, and what the measurements do not show. The tests are in `Tests/UITests/` and run in the `swift-build` workflow on a GitHub-hosted macOS runner. `scripts/summarize_ui_audit.py` turns the exported records into the tables below.

## Environment of the verified run

| Item | Value |
|---|---|
| Runner image | macOS 26.7.2 (25G332), GitHub-hosted `macos-26` |
| Xcode | 26.6 (17F113), Swift 6.3.3 |
| iOS Simulator SDK | 26.5 |
| Simulator | iPhone 17 Pro (402 x 874 pt) |
| Build | `xcodebuild` with `CODE_SIGNING_ALLOWED=NO`, destination `generic/platform=iOS Simulator` (build) and an iPhone 17 Pro simulator (tests) |
| Branch and commit | `v0.2.0-native-build`, run on commit `eb16279` |

No physical device was used.

## What the tests do

Fifteen simulator UI tests cover every screen: Dashboard, Device, Compatibility, Research, Package Catalog, Bootstrap Status, Diagnostics, Recovery, Settings and About. They also cover Korean text, the largest Dynamic Type size, and the Korean interface at that size. Each test launches its own app instance and terminates it in `tearDown`. Navigation waits for the destination's navigation bar and retries one tap, then fails.

Every screen is audited completely:

1. The test scrolls in steps of about a quarter of the screen. At each page it takes one screenshot and one snapshot of the accessibility tree.
2. A scroll that moves nothing is tried up to two more times before it counts as the end of the screen. A page believed to be last that still has text under the tab bar fails the audit as not fully audited.
3. Apple's `performAccessibilityAudit` runs on the first and the last page as two smaller audits, one for contrast and one for everything else. Each part has up to three attempts with a 3 second pause between them. A part that never completes is recorded as an explicit INCONCLUSIVE result that is counted and listed in the summary. It does not fail the test, and it is never a pass. The pixel analysis of the same screen is not affected.
4. Every text element on every page is measured by a pixel analyzer: the screenshot is cropped to the element, the dominant colour of the crop is taken as the effective background (so tints and translucency as rendered are included), the glyph colours are found, and WCAG 2.x contrast is computed. The crop is inset so rounded corners of a container are not read as text.
5. A screenshot of every page and a JSON record per screen are kept as test attachments.

## Categories

| Category | Meaning |
|---|---|
| PASS | Unobscured text with at least 4.5:1 contrast, measured on the part of the element between the system zones. |
| FAIL | Unobscured text below 4.5:1, measured on a fully visible element. |
| OCCLUDED | The element was only ever seen inside a system bar or its scroll-edge blur. |
| CLIPPED | Cut off by the screen edge, or glyph pixels touch the element's frame boundary. |
| INCONCLUSIVE | It could not be measured reliably. This includes an Apple finding that cannot be attached to an element, and a low reading from a thin slice of a rounded container. |

OCCLUDED and INCONCLUSIVE are never converted into PASS. An element that is occluded on one page and measured unobscured on another is counted by its measurement, and the occlusion events are still counted separately.

The system draws a soft blur beyond the bar frames while content scrolls under them. The analyzer treats the navigation bar and tab bar frames plus a 64 pt margin as a system zone, but only where content can scroll under a bar: no top margin on the first page, no bottom margin on the last page or on a screen that does not scroll.

## Results

Application text, from the verified run:

| Screen | Pages | PASS | FAIL | OCCLUDED | CLIPPED | INCONCLUSIVE |
|---|---|---|---|---|---|---|
| About | 1 | 6 | 0 | 0 | 0 | 0 |
| Bootstrap Status | 1 | 9 | 0 | 0 | 0 | 0 |
| Package Catalog | 3 | 23 | 0 | 0 | 0 | 3 |
| Compatibility | 2 | 16 | 0 | 0 | 0 | 0 |
| Dashboard | 1 | 33 | 0 | 0 | 0 | 0 |
| Dashboard (Korean) | 1 | 33 | 0 | 0 | 0 | 0 |
| Dashboard (Korean, largest text) | 11 | 33 | 0 | 0 | 0 | 1 |
| Dashboard (largest text) | 13 | 33 | 0 | 0 | 0 | 0 |
| Device | 1 | 36 | 0 | 0 | 0 | 0 |
| Diagnostics | 1 | 7 | 0 | 0 | 0 | 0 |
| Recovery | 6 | 23 | 0 | 0 | 0 | 0 |
| Research | 13 | 74 | 0 | 0 | 0 | 2 |
| Settings | 1 | 7 | 0 | 0 | 0 | 0 |
| **Total** | | **333** | **0** | **0** | **0** | **6** |

- All 13 audits reached the end of their screen.
- 312 occlusion events were seen across pages: an element inside a bar or blur zone on some page. Every such element was measured unobscured on another page.
- System-drawn bar text (tab bar and navigation bar) is reported apart, because it sits on a translucent glass gradient where a pixel reading is not reliable: PASS 49, INCONCLUSIVE 15. None failed.

### The six INCONCLUSIVE results

All six are findings of Apple's audit that it could not attach to an element: "Contrast failed" or "Potentially inaccessible text" with no addressable element, on Package Catalog (3 records), Research (2) and the Korean dashboard at the largest text size (1). They stand for 24 individual issues. They occur on the first and last page of the long screens, near the system bars. I could not identify the elements, so they are neither confirmed nor refuted. They are the main open item of this audit.

### Apple findings overruled by pixel evidence

Eight Apple findings were overruled. Each is kept in the record with both measurements.

| Screen | Apple finding | Evidence |
|---|---|---|
| Compatibility | Contrast failed: "Research Only" | pixel measurement 13.16:1 |
| Compatibility | Contrast failed: the Research Only explanation | 6.77:1 |
| Dashboard (largest text) | Contrast failed: "Status" | 15.41:1 |
| Recovery | Contrast failed: "Documented" | 6.09:1 |
| Recovery | Contrast failed: "Data loss" | 15.40:1 |
| Research | Contrast failed: "RF-009" | 15.40:1 |
| Research | Contrast failed: the statement on installable tools | 15.40:1 |
| Dashboard (Korean) | Text clipped: the Korean Research Only explanation | no glyph pixel touches the frame boundary; the text wraps completely |

For the Recovery findings the element screenshots Apple attached show crisp, legible text in the middle of the screen. The pixel analyzer's readings match the design tokens closely. `textPrimary` on a card measures 15.39:1 (designed 15.39:1), and `textSecondary` measures 7.92:1 (designed 7.89:1). That agreement over roughly 330 elements is why a conflict is settled in favour of the pixels. The risk of this choice is stated below.

## Defects found by this audit and fixed

- The provenance banner displayed a raw key ("provenance.test_fixture") because an interpolated `LocalizedStringKey` looked up a key that does not exist. The label is now looked up explicitly, with a safe fallback, and is localised in English and Korean.
- The Settings profile chooser was an inline wheel picker that truncated its options ("...A19 Pro, sa..."). It is now a list of rows with the full label, a 44 pt hit target and the selected trait. A test that appeared to check profile switching had been skipping its own tap. It now fails if the option is missing.
- Card headings were generic words repeated on every card ("Topic" seven times, "Package", "Finding"). They are now the real topic, package or finding names.
- A scroll on a busy runner could be dropped and end an audit early. End detection now confirms the end and checks for text left under the tab bar.

## Reliability of Apple's audit on the runner

Apple's audit intermittently stopped with "Audit failed to complete in time" (code -56) when the hosted runner was under load. It happened on three occasions, on Bootstrap, Device and Research, each time on both of the two attempts the test allowed then and always on the first audit after a launch. Every other test of those runs passed, and the same screens passed their audits in other runs of the same code, so this is treated as runner load and not as an app defect.

The first reaction was to fail the test, and that made CI fail at random. The audit is now split into a contrast part and an everything-else part, each with three bounded attempts, and a part that still does not complete is an INCONCLUSIVE result. That keeps the limit visible without letting an infrastructure timeout decide the build. The cost is real: a screen whose Apple audit never completes is still reported as audited by the pixel analyzer, but its Apple audit result is missing, and the summary lists it. A reader should check the INCONCLUSIVE list and not only the pass and fail counts.

A separate infrastructure event also occurred: on one run the macOS job was never acquired by a hosted runner ("not acquired by Runner of type hosted even after multiple attempts") during a GitHub capacity notice, so that commit has no UI result at all.

## Not covered, and limits of what was measured

- **VoiceOver** was not run. Only static properties were checked (labels, traits, headings, hit target sizes, Apple's audit). Reading order and spoken output were not tested.
- Only the default text size and the largest accessibility size (`AccessibilityXXXL`) were tested, in English and Korean. Sizes in between and the smaller accessibility sizes were not.
- Only portrait on one phone-sized simulator. iPad and landscape layouts were not tested, although the project allows them.
- Increase Contrast, Reduce Transparency, Reduce Motion, Bold Text and Light appearance were not tested. The app forces a dark appearance.
- The analyzer applies 4.5:1 to all text. The 3:1 allowance for large text is not used, which is stricter than WCAG. It measures a crop against its dominant colour, so a region that mixes backgrounds can mislead it. Elements with identical text are merged in the final result.
- Contrast is measured for text. Icons and other non-text contrast (WCAG 1.4.11) were not measured.
- A conflict between the analyzer and Apple's audit was settled by the analyzer. If the analyzer were systematically wrong in a way the design-token agreement does not reveal, those eight findings would be real. The screenshots do not suggest that.
- Rust/Swift parity is checked on shared vectors, not by running both implementations on the same inputs at random.
- No physical device was used at any point. Simulator results do not describe a device.
