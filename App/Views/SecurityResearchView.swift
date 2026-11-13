import SwiftUI
import HomeiKit

struct SecurityResearchView: View {
    @EnvironmentObject var model: AppModel

    var body: some View {
        Screen(titleKey: "research.title") {
            if let store = model.store {
                DataKindBanner(textKey: "research.kind_banner", symbol: "books.vertical")
                SectionCard(titleKey: "research.snapshot") {
                    InfoRow(labelKey: "research.snapshot_date", value: store.snapshotDate)
                    InfoRow(labelKey: "research.tracker_commit", value: store.trackerCommit)
                    Link("research.open_tracker", destination: URL(string: "https://github.com/PalanBanana/ios27-a17pro-jailbreak-tracker")!)
                        .frame(minHeight: 44, alignment: .leading)
                }
                if store.findings.isEmpty {
                    Text("state.empty_research").foregroundStyle(Theme.textSecondary)
                }
                ForEach(store.findings) { f in
                    SectionCard(titleKey: "research.finding") {
                        Text(f.title).font(.subheadline).foregroundStyle(Theme.textPrimary)
                        InfoRow(labelKey: "research.status", value: Bundle.main.localizedString(forKey: "finding.\(f.status.rawValue)", value: nil, table: nil))
                        InfoRow(labelKey: "research.scope", value: "\(f.hardwareScope) · \(f.firmwareScope)")
                        InfoRow(labelKey: "research.confidence", value: f.confidence)
                        Text("\(f.trackerClaimId) · \(f.evidenceIds.joined(separator: ", "))").font(.caption).foregroundStyle(Theme.textSecondary)
                    }
                }
            }
        }
    }
}
