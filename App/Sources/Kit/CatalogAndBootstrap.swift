import Foundation

public struct CatalogPackage: Codable, Equatable, Identifiable, Sendable {
    public struct Integrity: Codable, Equatable, Sendable { public let algorithm: String; public let digest: String; public let verified: Bool }
    public let id: String
    public let name: String
    public let version: String
    public let summary: String
    public let sourceId: String
    public let license: String
    public let integrity: Integrity
    public let dependencies: [String]
    public let fixture: Bool
    enum CodingKeys: String, CodingKey {
        case id, name, version, summary, license, integrity, dependencies, fixture
        case sourceId = "source_id"
    }
}

/// Offline package metadata. Nothing here can be installed.
public struct PackageCatalog: Sendable {
    public let packages: [CatalogPackage]
    private struct File: Decodable { let packages: [CatalogPackage] }

    public init(data: Data) throws { packages = try JSONDecoder().decode(File.self, from: data).packages }

    public static func bundled(_ bundle: Bundle = .main) throws -> PackageCatalog {
        guard let url = bundle.url(forResource: "catalog", withExtension: "json") else { throw DataStoreError.missing("catalog.json") }
        return try PackageCatalog(data: Data(contentsOf: url))
    }

    /// Transitive dependency summary, sorted and cycle-safe.
    public func dependencySummary(of id: String) -> [String] {
        var seen = Set<String>()
        var stack = packages.first { $0.id == id }?.dependencies ?? []
        while let next = stack.popLast() {
            if seen.insert(next).inserted { stack.append(contentsOf: packages.first { $0.id == next }?.dependencies ?? []) }
        }
        return seen.sorted()
    }

    /// Always false: this build cannot install anything.
    public func isInstallable(_ package: CatalogPackage) -> Bool { false }
}

public struct BootstrapComponent: Equatable, Sendable {
    public let id: String
    public let version: String
    public let dependsOn: [String]
    public init(id: String, version: String, dependsOn: [String]) { self.id = id; self.version = version; self.dependsOn = dependsOn }
}

/// Simulation only. Nothing is written, installed or changed.
public enum BootstrapSimulation {
    public static let banner = "SIMULATION ONLY. Nothing was installed, written or changed."

    /// Dependencies-first ordering. Returns nil for an unknown component or a cycle.
    public static func plan(_ components: [BootstrapComponent], requesting id: String) -> [BootstrapComponent]? {
        var done = Set<String>(), visiting = Set<String>(), out: [BootstrapComponent] = []
        func visit(_ id: String) -> Bool {
            if done.contains(id) { return true }
            guard visiting.insert(id).inserted, let c = components.first(where: { $0.id == id }) else { return false }
            for dep in c.dependsOn where !visit(dep) { return false }
            visiting.remove(id); done.insert(id); out.append(c)
            return true
        }
        return visit(id) ? out : nil
    }

    public static func statusLines(plan: [BootstrapComponent], decision: Decision) -> [String] {
        let tag = decision.status.isPositive ? "simulated" : "blocked"
        return [banner, "Compatibility: \(decision.status.label)"] + plan.map { "[\(tag)] \($0.id) \($0.version)" }
    }
}
