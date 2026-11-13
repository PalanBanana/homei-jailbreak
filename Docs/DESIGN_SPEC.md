# Application design specification

An original dark-blue interface. It states plainly what HOMEI is and is not.

## Principles

1. Research status, device profile, compatibility, safety, evidence and recovery come first.
2. A test fixture is never shown as a connected device. Every profile carries a provenance banner.
3. No enabled jailbreak control when no verified backend exists. A "Research Only" notice stands in its place.
4. No fake progress indicators and no success animations.
5. Colour is never the only signal. Every status has a label and an icon.

## Navigation

A tab bar with Dashboard, Device, Compatibility, Research and More. More lists Package Catalog, Bootstrap Status, Diagnostics, Recovery, Settings and About HOMEI. The ten sections of the brief map to these screens.

## Dashboard

Shows the provenance banner, the compatibility badge with the Research Only notice, the profile (product identifier, SoC family, firmware, build, architecture) and status (research readiness, last research update, safety).

## Colour tokens (`Theme.swift`)

| Token | RGB | Use |
|---|---|---|
| background | 0.04, 0.07, 0.13 | Screen background |
| surface | 0.07, 0.11, 0.20 | Cards |
| surfaceRaised | 0.10, 0.16, 0.28 | Notices |
| accent | 0.25, 0.62, 1.00 | Interactive and highlights |
| textPrimary | 0.93, 0.96, 1.00 | Body text |
| textSecondary | 0.62, 0.70, 0.82 | Secondary text |
| positive / caution / negative / neutral | green / amber / red / grey | Status |

Contrast was checked two ways. `scripts/check_contrast.py` computes WCAG ratios for every foreground and background pair the views draw, straight from `Theme.swift`: 27 of 27 pairs meet 4.5:1, and the thinnest is the negative-status badge text at 4.52:1. The simulator audit then measured the rendered text: see [UI_ACCESSIBILITY_AUDIT.md](UI_ACCESSIBILITY_AUDIT.md).

## Typography

System font with Dynamic Type. Headline for card titles, subheadline for body, footnote for notes, monospaced footnote for simulation output. No custom fonts.

## Components

`StatusBadge` (label + icon + colour), `SectionCard`, `InfoRow`, `ProvenanceBanner`, `ResearchOnlyNotice` (deliberately not a button) and `DataErrorView`.

## States

- **Empty:** research findings, catalog.
- **Error:** bundled data failing to load shows an explanatory screen, not a partial interface.
- **Unknown, not established, unsupported:** each has its own label, icon and colour.

## Accessibility

Rows and cards combine into single accessibility elements, status badges expose a text label, icons are decorative beside text, and layout uses system scaling. At accessibility text sizes a label and its value stack vertically instead of squeezing side by side. Card headings carry the header trait and are the real topic, package or finding names. Selectable rows are at least 44 pt tall and carry the selected trait.

Verified in the simulator: all ten screens audited page by page for contrast and clipping, the largest accessibility text size, Korean text, and Apple's audit. VoiceOver itself, device behaviour, Increase Contrast, Reduce Transparency, iPad and landscape were **not** tested. Results and limits are in [UI_ACCESSIBILITY_AUDIT.md](UI_ACCESSIBILITY_AUDIT.md).

Every screen says what kind of information it shows: the provenance banner on profile screens, a data-kind banner on Research and Package Catalog, and a legend on Device Information that lists observed data (none exists in this app), user-provided profile, synthetic fixture, verified research and unknown compatibility.

## Localisation

English primary, Korean secondary, through `Localizable.strings` (111 keys each, checked for parity in CI and by tests). Provenance labels are localised and a missing key falls back to a safe label, never a raw key. Research titles and explanations, recovery text, and compatibility explanations come from the English sources and are not translated yet.

## Branding

Name HOMEI Jailbreak, short name HOMEI, codename HOMEI-A17, tagline "Independent iOS Security and Jailbreak Framework". The icon (`App/Resources/Branding/homei-icon.svg`) is an original mark: an H inside a hexagon on a navy gradient. PNG sizes still need to be generated from it. The application does not claim affiliation with or endorsement by Apple or any jailbreak project.
