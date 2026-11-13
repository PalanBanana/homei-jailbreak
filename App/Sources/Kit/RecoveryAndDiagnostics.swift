import Foundation

public struct RecoveryTopic: Identifiable, Equatable, Sendable {
    public let id: String
    public let title: String
    public let documented: Bool
    public let body: String
}

public enum Recovery {
    public static let topics: [RecoveryTopic] = [
        .init(id: "firmware_signing", title: "Firmware signing", documented: false, body: "Apple decides which builds can be installed, and that can change without notice. Any signing status shown here is a dated third-party observation, not an Apple statement."),
        .init(id: "restore_limits", title: "Restore limitations", documented: true, body: "A restore installs a build that Apple is currently signing. If the installed build is no longer signed, it cannot be reinstalled after an erase."),
        .init(id: "backup_limits", title: "Backup limitations", documented: true, body: "A backup holds data and settings, not the operating system. A backup made on a newer iOS version generally cannot be restored to an older one. A backup does not guarantee a way back to an unsigned build."),
        .init(id: "data_loss", title: "Data loss", documented: true, body: "Erasing or restoring a device removes its data. Anything not backed up separately can be lost."),
        .init(id: "app_compatibility", title: "App compatibility", documented: false, body: "Apps, banking and payment features, and enterprise management may react to a modified system. HOMEI has no data about the effect on any specific app."),
        .init(id: "firmware_preservation", title: "Firmware preservation", documented: false, body: "Saved signing tickets (SHSH blobs) do not by themselves guarantee a restore. Whether a saved ticket can be used depends on the device, the build and tooling HOMEI does not evaluate."),
        .init(id: "recovery_uncertainty", title: "Device recovery uncertainty", documented: false, body: "No recovery path has been verified for the research target. HOMEI automates none."),
    ]

    /// Always NO-GO in this release: there is no backend to start.
    public static func goNoGo(for decision: Decision) -> (verdict: String, reasons: [String]) {
        var reasons = ["This release contains no operational backend. There is nothing to run.",
                       "No recovery path has been verified for this device and build."]
        if decision.status != .supported {
            reasons.append("Compatibility is \(decision.status.label). It needs exact-device, exact-build evidence.")
        }
        return ("NO-GO", reasons)
    }
}

public struct DiagnosticsReport: Codable, Sendable {
    public let product = "HOMEI"
    public let researchSnapshotDate: String
    public let trackerCommit: String
    public let profileProvenance: String
    public let compatibility: String
    public let explanation: [String]
    public let checks: [String: Bool]

    /// Local validation of the bundled data. Describes HOMEI only, never the device or other apps.
    public static func make(store: DataStore, decision: Decision) -> DiagnosticsReport {
        let evidenceIds = Set(store.evidence.map(\.id))
        let refsOk = store.declarations.allSatisfy { $0.evidenceIds.allSatisfy(evidenceIds.contains) }
            && store.findings.allSatisfy { $0.evidenceIds.allSatisfy(evidenceIds.contains) }
        let fixturesOk = store.fixtures.allSatisfy { $0.profile.provenance == .testFixture && (try? $0.profile.validate()) != nil }
        let noPositive = store.declarations.allSatisfy { !$0.status.isPositive }
        return DiagnosticsReport(
            researchSnapshotDate: store.snapshotDate, trackerCommit: store.trackerCommit,
            profileProvenance: decision.provenance.label, compatibility: decision.status.label,
            explanation: decision.explanation,
            checks: ["evidence_references_resolve": refsOk, "fixtures_are_test_fixtures": fixturesOk, "no_bundled_positive_state": noPositive])
    }

    /// JSON export. Refuses to export if any token looks like an identifier.
    public func export() -> String? {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        guard let data = try? encoder.encode(self), let json = String(data: data, encoding: .utf8) else { return nil }
        let separators = CharacterSet.alphanumerics.union(CharacterSet(charactersIn: "-")).inverted
        for token in json.components(separatedBy: separators) where token.count >= 14 && looksLikeIdentifier(token) { return nil }
        return json
    }
}
