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

    var body: some View {
        HStack(alignment: .firstTextBaseline) {
            Text(LocalizedStringKey(labelKey)).foregroundStyle(Theme.textSecondary)
            Spacer(minLength: 12)
            Text(value).foregroundStyle(Theme.textPrimary).multilineTextAlignment(.trailing)
        }
        .font(.subheadline)
        .accessibilityElement(children: .combine)
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
