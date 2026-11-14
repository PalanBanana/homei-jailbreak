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
    /// The sanitized lab state exported by the host controller. The app never runs a virtual machine.
    let lab: LabState?
    let labError: String?

    init() {
        var loadedStore: DataStore?
        var failure: String?
        do {
            loadedStore = try DataStore.bundled()
        } catch {
            failure = "\(error)"
        }
        store = loadedStore
        catalog = loadedStore == nil ? nil : try? PackageCatalog.bundled()
        loadError = failure
        do {
            lab = try LabStore.bundled()
            labError = nil
        } catch {
            lab = nil
            labError = "\(error)"
        }
    }

    var fixture: Fixture? { store?.fixtures.first { $0.id == selectedFixtureID } ?? store?.fixtures.first }

    var decision: Decision? {
        guard let store, let fixture else { return nil }
        return CompatibilityEngine.evaluate(profile: fixture.profile, declarations: store.declarations, evidence: store.evidence)
    }
}
