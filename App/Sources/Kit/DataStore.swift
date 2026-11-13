import Foundation

public struct Fixture: Codable, Equatable, Identifiable, Sendable {
    public var id: String
    public var label: String
    public var profile: DeviceProfile
}

public struct Finding: Codable, Equatable, Identifiable, Sendable {
    public var id: String
    public var title: String
    public var status: String
    public var evidenceIds: [String]
    public var cveIds: [String]
    public var hardwareScope: String
    public var firmwareScope: String
    public var trackerClaimId: String
    public var lastChecked: String
    public var confidence: String
    enum CodingKeys: String, CodingKey {
        case id, title, status, confidence
        case evidenceIds = "evidence_ids", cveIds = "cve_ids", hardwareScope = "hardware_scope"
        case firmwareScope = "firmware_scope", trackerClaimId = "tracker_claim_id", lastChecked = "last_checked"
    }
}

public enum DataStoreError: Error, CustomStringConvertible {
    case missing(String)
    case invalid(String, String)
    public var description: String {
        switch self {
        case .missing(let f): return "Missing bundled file \(f)"
        case .invalid(let f, let why): return "Invalid \(f): \(why)"
        }
    }
}

/// Offline, bundled research data. The loader is injected so the app reads from its bundle and
/// tests read from the repository's Data directory.
public struct DataStore: Sendable {
    public let evidence: [Evidence]
    public let declarations: [Declaration]
    public let findings: [Finding]
    public let fixtures: [Fixture]
    public let snapshotDate: String
    public let trackerCommit: String

    private struct EvidenceFile: Decodable { let evidence: [Evidence]; let snapshot_date: String; let tracker_commit: String }
    private struct DeclFile: Decodable { let declarations: [Declaration] }
    private struct ResearchFile: Decodable { let findings: [Finding] }
    private struct FixtureFile: Decodable { let fixtures: [Fixture] }

    public init(load: (String) throws -> Data) throws {
        func decode<T: Decodable>(_ name: String, _ type: T.Type) throws -> T {
            let data: Data
            do { data = try load(name) } catch { throw DataStoreError.missing(name) }
            do { return try JSONDecoder().decode(T.self, from: data) }
            catch { throw DataStoreError.invalid(name, "\(error)") }
        }
        let e = try decode("evidence.json", EvidenceFile.self)
        evidence = e.evidence
        snapshotDate = e.snapshot_date
        trackerCommit = e.tracker_commit
        declarations = try decode("compatibility.json", DeclFile.self).declarations
        findings = try decode("research-index.json", ResearchFile.self).findings
        fixtures = try decode("device-fixtures.json", FixtureFile.self).fixtures
    }

    /// Load from an app bundle that contains the four Data files.
    public static func bundled(_ bundle: Bundle = .main) throws -> DataStore {
        try DataStore { name in
            let parts = name.split(separator: ".").map(String.init)
            guard let url = bundle.url(forResource: parts[0], withExtension: parts[1]) else { throw DataStoreError.missing(name) }
            return try Data(contentsOf: url)
        }
    }
}
