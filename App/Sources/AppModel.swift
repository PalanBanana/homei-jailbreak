import SwiftUI
import HomeiKit

/// App state. Holds a selected profile and its decision. There is no device discovery, no
/// pairing and no polling: profiles come from bundled fixtures.
@MainActor
final class AppModel: ObservableObject {
    @Published var selectedFixtureID: String = "FIX-A"
    let store: DataStore?
    let catalog: PackageCatalog?
    let loadError: String?

    init() {
        do {
            store = try DataStore.bundled()
            catalog = try? PackageCatalog.bundled()
            loadError = nil
        } catch {
            store = nil
            catalog = nil
            loadError = "\(error)"
        }
    }

    var fixture: Fixture? { store?.fixtures.first { $0.id == selectedFixtureID } ?? store?.fixtures.first }

    var decision: Decision? {
        guard let store, let fixture else { return nil }
        return CompatibilityEngine.evaluate(profile: fixture.profile, declarations: store.declarations, evidence: store.evidence)
    }
}
