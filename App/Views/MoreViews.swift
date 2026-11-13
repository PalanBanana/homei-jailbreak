import SwiftUI
import HomeiKit

struct PackageCatalogView: View {
    @EnvironmentObject var model: AppModel

    var body: some View {
        Screen(titleKey: "catalog.title") {
            DataKindBanner(textKey: "catalog.kind_banner", symbol: "shippingbox")
            Text("catalog.metadata_only").font(.subheadline).foregroundStyle(Theme.textSecondary)
            if let catalog = model.catalog {
                ForEach(catalog.packages) { p in
                    SectionCard(titleKey: "catalog.package") {
                        Text("\(p.name) \(p.version)").font(.headline).foregroundStyle(Theme.textPrimary)
                        Text(p.summary).font(.subheadline).foregroundStyle(Theme.textSecondary)
                        InfoRow(labelKey: "catalog.license", value: p.license)
                        InfoRow(labelKey: "catalog.source", value: p.sourceId)
                        InfoRow(labelKey: "catalog.integrity", value: p.integrity.verified ? "sha256 ✓" : String(localized: "catalog.integrity_unverified"))
                        InfoRow(labelKey: "catalog.dependencies", value: catalog.dependencySummary(of: p.id).joined(separator: ", "))
                        Label("catalog.not_installable", systemImage: "nosign").font(.footnote).foregroundStyle(Theme.caution)
                    }
                }
            } else {
                Text("state.empty_catalog").foregroundStyle(Theme.textSecondary)
            }
        }
    }
}

struct BootstrapStatusView: View {
    @EnvironmentObject var model: AppModel
    private let components = [
        BootstrapComponent(id: "fixture.runtime", version: "1.0.0", dependsOn: []),
        BootstrapComponent(id: "fixture.package-manager", version: "0.9.0", dependsOn: ["fixture.runtime"]),
        BootstrapComponent(id: "fixture.core-tools", version: "1.2.0", dependsOn: ["fixture.runtime"]),
        BootstrapComponent(id: "fixture.catalog-client", version: "0.3.0", dependsOn: ["fixture.package-manager", "fixture.core-tools"]),
    ]

    var body: some View {
        Screen(titleKey: "bootstrap.title") {
            Label("bootstrap.simulation_only", systemImage: "testtube.2").font(.headline).foregroundStyle(Theme.accent)
            if let d = model.decision { ProvenanceBanner(provenance: d.provenance) }
            if let d = model.decision, let plan = BootstrapSimulation.plan(components, requesting: "fixture.catalog-client") {
                SectionCard(titleKey: "bootstrap.mock_status") {
                    ForEach(BootstrapSimulation.statusLines(plan: plan, decision: d), id: \.self) {
                        Text($0).font(.system(.footnote, design: .monospaced)).foregroundStyle(Theme.textPrimary)
                    }
                }
            }
        }
    }
}

struct DiagnosticsView: View {
    @EnvironmentObject var model: AppModel

    var body: some View {
        Screen(titleKey: "diagnostics.title") {
            if let store = model.store, let d = model.decision {
                let report = DiagnosticsReport.make(store: store, decision: d)
                ProvenanceBanner(provenance: d.provenance)
                SectionCard(titleKey: "diagnostics.checks") {
                    ForEach(report.checks.sorted(by: { $0.key < $1.key }), id: \.key) { item in
                        Label(item.key, systemImage: item.value ? "checkmark.circle" : "xmark.circle")
                            .foregroundStyle(item.value ? Theme.positive : Theme.negative).font(.footnote)
                            .accessibilityLabel(Text("\(item.key): \(item.value ? "passed" : "failed")"))
                    }
                }
                Text("diagnostics.sanitized_note").font(.footnote).foregroundStyle(Theme.textSecondary)
                if let json = report.export() {
                    ShareLink("diagnostics.export", item: json).frame(minHeight: 44)
                }
            }
        }
    }
}

struct RecoveryView: View {
    @EnvironmentObject var model: AppModel

    var body: some View {
        Screen(titleKey: "recovery.title") {
            if let d = model.decision {
                ProvenanceBanner(provenance: d.provenance)
                let g = Recovery.goNoGo(for: d)
                SectionCard(titleKey: "recovery.no_go") {
                    Text(g.verdict).font(.title2.bold()).foregroundStyle(Theme.negative)
                    ForEach(g.reasons, id: \.self) { Text("• \($0)").font(.subheadline).foregroundStyle(Theme.textPrimary) }
                }
            }
            ForEach(Recovery.topics) { t in
                SectionCard(titleKey: "recovery.topic") {
                    Text(t.title).font(.headline).foregroundStyle(Theme.textPrimary)
                    Text(t.body).font(.subheadline).foregroundStyle(Theme.textSecondary)
                    Text(t.documented ? "recovery.documented" : "recovery.uncertain").font(.caption).foregroundStyle(Theme.accent)
                }
            }
            Text("recovery.no_guarantee").font(.footnote).foregroundStyle(Theme.caution)
        }
    }
}

struct SettingsView: View {
    @EnvironmentObject var model: AppModel

    var body: some View {
        Screen(titleKey: "settings.title") {
            if let store = model.store {
                SectionCard(titleKey: "settings.fixture_picker") {
                    Picker("settings.fixture_picker", selection: $model.selectedFixtureID) {
                        ForEach(store.fixtures) { Text($0.label).tag($0.id) }
                    }
                    .pickerStyle(.inline)
                }
                if let fixture = model.fixture { ProvenanceBanner(provenance: fixture.profile.provenance) }
            }
            Text("settings.no_background_discovery").font(.footnote).foregroundStyle(Theme.textSecondary)
            Text("settings.language_note").font(.footnote).foregroundStyle(Theme.textSecondary)
        }
    }
}

struct AboutView: View {
    var body: some View {
        Screen(titleKey: "about.title") {
            Text("app.name").font(.title.bold()).foregroundStyle(Theme.textPrimary)
            Text("app.stage_notice").foregroundStyle(Theme.textSecondary)
            SectionCard(titleKey: "about.not_affiliated") {
                Text("about.not_affiliated_detail").font(.subheadline).foregroundStyle(Theme.textSecondary)
            }
            SectionCard(titleKey: "about.license") {
                Text("about.license_detail").font(.subheadline).foregroundStyle(Theme.textSecondary)
            }
        }
    }
}
