import SwiftUI
import HomeiKit

struct CompatibilityView: View {
    @EnvironmentObject var model: AppModel

    var body: some View {
        Screen(titleKey: "compat.title") {
            if let d = model.decision, let store = model.store {
                StatusBadge(status: d.status)
                ResearchOnlyNotice()
                SectionCard(titleKey: "compat.explanation") {
                    ForEach(d.explanation, id: \.self) { Text($0).font(.subheadline).foregroundStyle(Theme.textPrimary) }
                }
                SectionCard(titleKey: "compat.evidence_required") {
                    Text("compat.evidence_required_detail").font(.subheadline).foregroundStyle(Theme.textSecondary)
                }
                if let id = d.matchedDeclaration, let decl = store.declarations.first(where: { $0.id == id }) {
                    SectionCard(titleKey: "compat.cited_evidence") {
                        ForEach(decl.evidenceIds, id: \.self) { eid in
                            if let e = store.evidence.first(where: { $0.id == eid }) {
                                VStack(alignment: .leading, spacing: 2) {
                                    Text("\(e.id) · \(e.evidenceClass.rawValue)").font(.caption).foregroundStyle(Theme.accent)
                                    Text(e.title).font(.subheadline).foregroundStyle(Theme.textPrimary)
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
