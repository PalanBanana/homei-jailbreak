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

Contrast ratios were **not** measured. Verify them before release.

## Typography

System font with Dynamic Type. Headline for card titles, subheadline for body, footnote for notes, monospaced footnote for simulation output. No custom fonts.

## Components

`StatusBadge` (label + icon + colour), `SectionCard`, `InfoRow`, `ProvenanceBanner`, `ResearchOnlyNotice` (deliberately not a button) and `DataErrorView`.

## States

- **Empty:** research findings, catalog.
- **Error:** bundled data failing to load shows an explanatory screen, not a partial interface.
- **Unknown, not established, unsupported:** each has its own label, icon and colour.

## Accessibility

Rows and cards combine into single accessibility elements, status badges expose a text label, icons are decorative beside text, and layout uses system scaling. VoiceOver and Dynamic Type behaviour has **not** been tested on a device or simulator.

## Localisation

English primary, Korean secondary, through `Localizable.strings` (91 keys each, checked for parity in CI). Research titles and explanations come from the English research tracker and are not translated.

## Branding

Name HOMEI Jailbreak, short name HOMEI, codename HOMEI-A17, tagline "Independent iOS Security and Jailbreak Framework". The icon (`App/Resources/Branding/homei-icon.svg`) is an original mark: an H inside a hexagon on a navy gradient. PNG sizes still need to be generated from it. The application does not claim affiliation with or endorsement by Apple or any jailbreak project.
