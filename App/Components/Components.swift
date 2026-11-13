import SwiftUI
import HomeiKit

/// Status label with colour and text. Colour is never the only signal.
struct StatusBadge: View {
    let status: CompatibilityStatus

    private var color: Color {
        switch status {
        case .supported, .partiallySupported: return Theme.positive
        case .notEstablished: return Theme.caution
        case .unsupported: return Theme.negative
        case .unknown: return Theme.neutral
        }
    }

    private var symbol: String {
        switch status {
        case .supported: return "checkmark.seal"
        case .partiallySupported: return "circle.lefthalf.filled"
        case .notEstablished: return "questionmark.diamond"
        case .unsupported: return "xmark.octagon"
        case .unknown: return "circle.dotted"
        }
    }

    var body: some View {
        Label(LocalizedStringKey(status.localizationKey), systemImage: symbol)
            .font(.subheadline.weight(.semibold))
            .padding(.horizontal, 12).padding(.vertical, 6)
            .foregroundStyle(color)
            .background(color.opacity(0.16), in: Capsule())
            .accessibilityLabel(Text(LocalizedStringKey(status.localizationKey)))
    }
}

struct SectionCard<Content: View>: View {
    let titleKey: String
    @ViewBuilder var content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(LocalizedStringKey(titleKey)).font(.headline).foregroundStyle(Theme.textPrimary)
                .accessibilityAddTraits(.isHeader)
            content
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(Theme.spacing)
        .background(Theme.surface, in: RoundedRectangle(cornerRadius: Theme.radius))
        .accessibilityElement(children: .contain)
    }
}

struct InfoRow: View {
    let labelKey: String
    let value: String
    @Environment(\.dynamicTypeSize) private var typeSize

    var body: some View {
        // At accessibility text sizes the label and value stack instead of squeezing side by side.
        Group {
            if typeSize.isAccessibilitySize {
                VStack(alignment: .leading, spacing: 2) { label; valueText }
            } else {
                HStack(alignment: .firstTextBaseline) {
                    label
                    Spacer(minLength: 12)
                    valueText.multilineTextAlignment(.trailing)
                }
            }
        }
        .font(.subheadline)
        .accessibilityElement(children: .combine)
    }

    private var label: some View { Text(LocalizedStringKey(labelKey)).foregroundStyle(Theme.textSecondary) }
    private var valueText: some View { Text(value).foregroundStyle(Theme.textPrimary) }
}

/// Says what kind of data a screen shows when it is not a profile.
struct DataKindBanner: View {
    let textKey: String
    let symbol: String

    var body: some View {
        Label(LocalizedStringKey(textKey), systemImage: symbol)
            .font(.footnote)
            .foregroundStyle(Theme.accent)
            .padding(10)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(Theme.accent.opacity(0.12), in: RoundedRectangle(cornerRadius: 10))
    }
}

/// The five kinds of information HOMEI keeps apart. Observed data is listed to say it never exists here.
struct DataKindsLegend: View {
    var body: some View {
        SectionCard(titleKey: "legend.title") {
            InfoRow(labelKey: "legend.observed", value: String(localized: "legend.observed_value"))
            InfoRow(labelKey: "legend.user_profile", value: String(localized: "legend.user_profile_value"))
            InfoRow(labelKey: "legend.fixture", value: String(localized: "legend.fixture_value"))
            InfoRow(labelKey: "legend.research", value: String(localized: "legend.research_value"))
            InfoRow(labelKey: "legend.unknown", value: String(localized: "legend.unknown_value"))
        }
    }
}

/// Banner that says the profile is not a connected device.
struct ProvenanceBanner: View {
    let provenance: Provenance

    var body: some View {
        Label(provenance.label, systemImage: "doc.text.magnifyingglass")
            .font(.footnote)
            .foregroundStyle(Theme.accent)
            .padding(10)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(Theme.accent.opacity(0.12), in: RoundedRectangle(cornerRadius: 10))
    }
}

/// Stands in for a jailbreak button. It is deliberately not a button.
struct ResearchOnlyNotice: View {
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Label("compat.research_only", systemImage: "lock.shield").font(.headline)
            Text("compat.research_only_detail").font(.footnote).foregroundStyle(Theme.textSecondary)
        }
        .foregroundStyle(Theme.textPrimary)
        .padding(Theme.spacing)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Theme.surfaceRaised, in: RoundedRectangle(cornerRadius: Theme.radius))
        .accessibilityElement(children: .combine)
    }
}

struct DataErrorView: View {
    let message: String

    var body: some View {
        VStack(spacing: 8) {
            Image(systemName: "exclamationmark.triangle").font(.largeTitle).foregroundStyle(Theme.caution)
            Text("state.error_data").font(.headline)
            Text(message).font(.footnote).foregroundStyle(Theme.textSecondary).multilineTextAlignment(.center)
        }
        .padding()
    }
}
