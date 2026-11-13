import SwiftUI
import HomeiKit

struct DashboardView: View {
    @EnvironmentObject var model: AppModel

    var body: some View {
        Screen(titleKey: "app.name") {
            if let fixture = model.fixture, let decision = model.decision, let store = model.store {
                Text("app.tagline").font(.subheadline).foregroundStyle(Theme.textSecondary)
                ProvenanceBanner(provenance: fixture.profile.provenance)
                SectionCard(titleKey: "dashboard.compatibility") {
                    StatusBadge(status: decision.status)
                    ResearchOnlyNotice()
                }
                SectionCard(titleKey: "dashboard.device") {
                    InfoRow(labelKey: "dashboard.product_type", value: fixture.profile.productType)
                    InfoRow(labelKey: "dashboard.soc", value: fixture.profile.socFamily?.rawValue ?? "-")
                    InfoRow(labelKey: "dashboard.firmware", value: fixture.profile.productVersion)
                    InfoRow(labelKey: "dashboard.build", value: fixture.profile.buildVersion)
                    InfoRow(labelKey: "dashboard.architecture", value: fixture.profile.cpuArchitecture ?? "-")
                }
                SectionCard(titleKey: "dashboard.status") {
                    InfoRow(labelKey: "dashboard.readiness", value: "LEVEL 0 / NO-GO")
                    InfoRow(labelKey: "dashboard.last_update", value: store.snapshotDate)
                    InfoRow(labelKey: "dashboard.safety", value: String(localized: "dashboard.safety_value"))
                }
            }
        }
    }
}
