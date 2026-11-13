import SwiftUI

struct RootView: View {
    @EnvironmentObject var model: AppModel

    var body: some View {
        Group {
            if let error = model.loadError {
                DataErrorView(message: error)
            } else {
                TabView {
                    NavigationStack { DashboardView() }.tabItem { Label("tab.dashboard", systemImage: "gauge.with.dots.needle.50percent") }
                    NavigationStack { DeviceInfoView() }.tabItem { Label("tab.device", systemImage: "iphone") }
                    NavigationStack { CompatibilityView() }.tabItem { Label("tab.compatibility", systemImage: "checkmark.shield") }
                    NavigationStack { SecurityResearchView() }.tabItem { Label("tab.research", systemImage: "doc.text.magnifyingglass") }
                    NavigationStack { MoreView() }.tabItem { Label("tab.more", systemImage: "ellipsis.circle") }
                }
                .tint(Theme.accent)
            }
        }
        .background(Theme.background.ignoresSafeArea())
    }
}

struct MoreView: View {
    var body: some View {
        List {
            NavigationLink("section.package_catalog") { PackageCatalogView() }
            NavigationLink("section.bootstrap") { BootstrapStatusView() }
            NavigationLink("section.diagnostics") { DiagnosticsView() }
            NavigationLink("section.recovery") { RecoveryView() }
            NavigationLink("section.settings") { SettingsView() }
            NavigationLink("section.about") { AboutView() }
        }
        .scrollContentBackground(.hidden)
        .background(Theme.background)
        .navigationTitle("tab.more")
    }
}

/// Shared scaffold: scrolling column on the HOMEI background.
struct Screen<Content: View>: View {
    let titleKey: String
    @ViewBuilder var content: Content

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: Theme.spacing) { content }
                .padding(Theme.spacing)
        }
        .background(Theme.background)
        .navigationTitle(LocalizedStringKey(titleKey))
        .navigationBarTitleDisplayMode(.inline)
    }
}
