import Foundation

// Firmware research models. They mirror `Lab/src/analysis/research.rs` (the analysis bundle),
// `Lab/src/feasibility.rs` (the matrix) and `scripts/export_sptm_census.py` (the register summary).
//
// The app never reads firmware, never decrypts anything and never runs a virtual machine. It reads
// the sanitized JSON files that the host tools wrote, and it checks the same invariants the Rust
// tests check. A file that fails a check is an error state, never a partially shown result.

// MARK: Vocabularies

/// What is known about one manifest component. Each case has its own localized explanation.
public enum ResearchAvailability: String, Codable, CaseIterable, Sendable {
    case analysed
    case containerOnly = "container_only"
    case encrypted
    case unsupported
    case unavailable
    case missing
    case noManifestPath = "no_manifest_path"

    public var localizationKey: String { "research.availability.\(rawValue)" }
}

/// The evidence behind a statement. Only `target` and `manifest` may be stated as facts.
public enum ResearchConfidence: String, Codable, CaseIterable, Sendable {
    case target, manifest, inferred, unknown

    public var tag: String {
        switch self {
        case .target: return "T"
        case .manifest: return "M"
        case .inferred: return "I"
        case .unknown: return "U"
        }
    }

    public var isFact: Bool { self == .target || self == .manifest }
}

/// The five feasibility statuses. No other value is valid.
public enum ResearchStatus: String, Codable, CaseIterable, Sendable {
    case verified = "VERIFIED"
    case partial = "PARTIAL"
    case blocked = "BLOCKED"
    case unsupported = "UNSUPPORTED"
    case unverified = "UNVERIFIED"

    public var localizationKey: String { "research.status.\(rawValue)" }
}

// MARK: Analysis bundle

public struct ResearchTarget: Codable, Equatable, Sendable {
    public let label: String
    public let productType: String
    public let boardConfig: String
    public let chip: String
    public let architecture: String
    public let productVersion: String
    public let build: String
    public let eraseVariant: String
}

public struct ResearchIdentityCheck: Codable, Equatable, Sendable {
    public let id: String
    public let expected: String
    public let observed: String
    public let passed: Bool
}

public struct ResearchManifestIdentity: Codable, Equatable, Sendable {
    public let variant: String
    public let restoreBehavior: String
    public let boardIdHex: String
    public let chipIdHex: String
    public let deviceClass: String
    public let componentCount: Int
}

public struct ResearchInput: Codable, Equatable, Sendable {
    public let role: String
    public let fileName: String
    public let bytes: Int64
    public let sha256: String
}

public struct ResearchProvenance: Codable, Equatable, Sendable {
    public let tool: String
    public let toolVersion: String
    public let schemaVersion: Int
    public let analysedOn: String
    public let inputs: [ResearchInput]
    public let externalSteps: [String]
}

public struct ResearchArchiveEntry: Codable, Equatable, Sendable {
    public let name: String
    public let method: Int
    public let compressedBytes: Int64
    public let uncompressedBytes: Int64
}

public struct ResearchArchive: Codable, Equatable, Sendable {
    public let entryCount: Int
    public let storedEntries: Int
    public let deflatedEntries: Int
    public let uncompressedTotalBytes: Int64
    public let entries: [ResearchArchiveEntry]
    public let eraseComponentsMissingFromArchive: [String]
}

/// The outer container of a local file, as classified without decompression.
public struct ResearchContainer: Codable, Equatable, Sendable {
    public let kind: String
    public let fourcc: String?
    public let description: String?
    public let payloadBytes: Int64?
    public let payloadCompression: String?
    public let keybagPresent: Bool?
    public let keybagBytes: Int64?
}

public struct ResearchComponent: Codable, Equatable, Sendable, Identifiable {
    public let name: String
    public let path: String?
    public let trusted: Bool
    public let inArchive: Bool
    public let archiveUncompressedBytes: Int64?
    public let localFile: String?
    public let localBytes: Int64?
    public let localSha256: String?
    public let manifestDigestMatches: Bool?
    public let container: ResearchContainer?
    public let availability: ResearchAvailability
    public let payloadFile: String?
    public let payloadBytes: Int64?
    public let payloadSha256: String?

    public var id: String { name }
}

public struct ResearchSummary: Codable, Equatable, Sendable {
    public let eraseComponents: Int
    public let analysed: Int
    public let containerOnly: Int
    public let encrypted: Int
    public let unsupported: Int
    public let unavailable: Int
    public let missing: Int
    public let noManifestPath: Int

    public var total: Int {
        analysed + containerOnly + encrypted + unsupported + unavailable + missing + noManifestPath
    }
}

public struct ResearchSecurityState: Codable, Equatable, Sendable {
    public let role: String
    public let component: String
    public let availability: ResearchAvailability
    public let encryption: String
    public let payloadCompression: String?
}

/// Addresses are unsigned 64-bit. Kernel-space addresses exceed `Int64.max`, so they are not `Int64`.
public struct ResearchSegment: Codable, Equatable, Sendable {
    public let name: String
    public let vmaddr: UInt64
    public let vmsize: UInt64
    public let fileoff: UInt64
    public let filesize: UInt64
    public let sections: Int
}

public struct ResearchMacho: Codable, Equatable, Sendable {
    public let cputype: String
    public let cpusubtypeHex: String
    public let architecture: String
    public let filetype: String
    public let loadCommandCount: Int
    public let segments: [ResearchSegment]
    public let filesetEntries: [String]
    public let buildPlatform: Int?
    public let minimumOs: String?
}

public struct ResearchKernelcache: Equatable, Sendable, Decodable {
    public let payloadFile: String
    public let payloadBytes: Int64
    /// The parsed header and load commands, or nil when parsing failed.
    public let macho: ResearchMacho?
    /// The reason parsing failed, or nil when it succeeded.
    public let machoError: String?
    public let filesetEntryCount: Int
    public let filesetEntries: [String]
    public let xnuVersion: String?
    public let configurationToken: String?

    private enum CodingKeys: String, CodingKey {
        case payloadFile, payloadBytes, macho, filesetEntryCount, filesetEntries, xnuVersion, configurationToken
    }

    /// Rust writes a `Result` as an externally tagged value: `{"Ok": ...}` or `{"Err": "..."}`.
    private struct MachoResult: Decodable {
        let ok: ResearchMacho?
        let err: String?
        private enum CodingKeys: String, CodingKey { case ok = "Ok", err = "Err" }
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        payloadFile = try c.decode(String.self, forKey: .payloadFile)
        payloadBytes = try c.decode(Int64.self, forKey: .payloadBytes)
        let result = try c.decode(MachoResult.self, forKey: .macho)
        macho = result.ok
        machoError = result.err
        filesetEntryCount = try c.decode(Int.self, forKey: .filesetEntryCount)
        filesetEntries = try c.decode([String].self, forKey: .filesetEntries)
        xnuVersion = try c.decodeIfPresent(String.self, forKey: .xnuVersion)
        configurationToken = try c.decodeIfPresent(String.self, forKey: .configurationToken)
    }
}

public struct ResearchRegEntry: Codable, Equatable, Sendable {
    public let addressHex: String
    public let sizeHex: String
}

public struct ResearchRangeEntry: Codable, Equatable, Sendable {
    public let childBaseHex: String
    public let parentBaseHex: String
    public let sizeHex: String
}

public struct ResearchAIC: Codable, Equatable, Sendable {
    public let path: String
    public let compatible: [String]
    public let interruptCells: Int?
    public let reg: [ResearchRegEntry]
    public let regNamesPresent: Bool
    public let translatedBaseHex: String?
    public let translationSource: String?
    public let offsetAndStrideProperties: [String]
    public let aicIackOffsetInTree: Bool
    public let consumersSingleCell: Int
    public let consumersMultiElement: Int
}

public struct ResearchPMGR: Codable, Equatable, Sendable {
    public let path: String
    public let compatible: [String]
    public let regWindows: Int
    public let powerDomainNames: [String]
    public let interrupts: [Int]
}

public struct ResearchDeviceTree: Codable, Equatable, Sendable {
    public let payloadFile: String
    public let payloadBytes: Int64
    public let payloadSha256: String
    public let nodeCount: Int
    public let rootCompatible: [String]
    public let rootModel: String?
    public let rootTargetType: String?
    public let armIoCompatible: [String]
    public let armIoRanges: [ResearchRangeEntry]
    public let aic: ResearchAIC?
    public let pmgr: ResearchPMGR?
    public let aicTimebaseReg: [ResearchRegEntry]
}

public struct ResearchGhidraName: Codable, Equatable, Sendable {
    public let name: String
    public let presentInIndex: Bool
}

public struct ResearchGhidra: Codable, Equatable, Sendable {
    public let projectName: String
    public let projectFileBytes: Int64
    public let projectFileState: String
    public let repositoryPresent: Bool
    public let projectPropertiesPresent: Bool
    public let indexFiles: Int
    public let indexBytesScanned: Int64
    public let names: [ResearchGhidraName]
    public let functionCount: Int?
    public let exportPresent: Bool
}

public struct ResearchClaim: Codable, Equatable, Sendable, Identifiable {
    public let id: String
    public let statement: String
    public let confidence: ResearchConfidence
    public let source: String
    public let limitation: String
}

public struct ResearchAnalysis: Decodable, Equatable, Sendable {
    public let schemaVersion: Int
    public let target: ResearchTarget
    public let identityChecks: [ResearchIdentityCheck]
    public let manifestIdentities: [ResearchManifestIdentity]
    public let provenance: ResearchProvenance
    public let archive: ResearchArchive
    public let components: [ResearchComponent]
    public let summary: ResearchSummary
    public let security: [ResearchSecurityState]
    public let kernelcache: ResearchKernelcache?
    public let devicetree: ResearchDeviceTree?
    public let ghidra: ResearchGhidra?
    public let claims: [ResearchClaim]
}

// MARK: Feasibility matrix

public struct ResearchDecision: Codable, Equatable, Sendable {
    public let statement: String
    public let source: String
    public let evidenceType: String
    public let confidence: ResearchConfidence
    public let lastVerified: String
    public let limitation: String
}

public struct ResearchFeasibilityComponent: Codable, Equatable, Sendable, Identifiable {
    public let id: String
    public let title: String
    public let question: String
    public let status: ResearchStatus
    public let basis: String
    public let decisions: [ResearchDecision]
}

public struct ResearchExactVm: Codable, Equatable, Sendable {
    public let status: String
    public let statement: String
}

public struct ResearchMatrix: Codable, Equatable, Sendable {
    public let schemaVersion: Int
    public let target: String
    public let generatedOn: String
    public let analysisAnalysedOn: String
    public let exactTargetVm: ResearchExactVm
    public let components: [ResearchFeasibilityComponent]
}

// MARK: SPTM register census summary

public struct ResearchRegister: Codable, Equatable, Sendable, Identifiable {
    public let encoding: String
    public let definedInFork: Bool
    public let accesses: Int
    public let mrs: Int
    public let msr: Int
    public let readModifyWriteCandidates: Int
    public let reachableFromEarlyExceptionVectors: Int
    public let callGraphClasses: [String]
    public let lifetimeClasses: [String]

    public var id: String { encoding }
}

public struct ResearchCensus: Codable, Equatable, Sendable {
    public let schemaVersion: Int
    public let sourceDescription: String
    public let accessRows: Int
    public let registerCount: Int
    public let definedInForkCount: Int
    public let undefinedInForkCount: Int
    public let runtimeReachableRegisterCount: Int
    public let registers: [ResearchRegister]
}

// MARK: Loading and checks

public enum ResearchStore {
    /// The host tools write snake_case keys. Every model here uses camelCase names.
    public static let decoder: JSONDecoder = {
        let d = JSONDecoder()
        d.keyDecodingStrategy = .convertFromSnakeCase
        return d
    }()

    /// The first broken invariant, or nil. These are the Swift counterparts of the Rust tests.
    public static func violation(_ a: ResearchAnalysis) -> String? {
        if a.target.productType != "iPhone16,2" || a.target.boardConfig != "D84AP" || a.target.chip != "T8130" {
            return "target identity is not the fixed target"
        }
        if a.target.build != "24A446" || a.target.productVersion != "27.0.1" {
            return "build or version is not the fixed target"
        }
        if let failed = a.identityChecks.first(where: { !$0.passed }) {
            return "identity check failed: \(failed.id)"
        }
        if a.components.count != a.summary.eraseComponents {
            return "component count does not match the summary"
        }
        if a.summary.total != a.summary.eraseComponents {
            return "summary categories do not add up to the component count"
        }
        for c in a.components {
            let keybag = c.container?.keybagPresent == true
            if keybag != (c.availability == .encrypted) {
                return "availability of \(c.name) disagrees with its keybag element"
            }
        }
        return nil
    }

    /// The first broken invariant of a matrix, or nil.
    public static func violation(_ m: ResearchMatrix) -> String? {
        if m.components.count != 16 {
            return "matrix must have sixteen components"
        }
        if m.exactTargetVm.status != "NOT_ESTABLISHED" {
            return "matrix must not claim an exact target VM"
        }
        if let v = m.components.first(where: { $0.status == .verified }) {
            return "\(v.id) is VERIFIED without a boot"
        }
        if let empty = m.components.first(where: { $0.decisions.isEmpty }) {
            return "\(empty.id) has no stated decision"
        }
        return nil
    }

    /// The first broken invariant of the census summary, or nil.
    public static func violation(_ c: ResearchCensus) -> String? {
        if c.registerCount != c.registers.count {
            return "register count does not match the register list"
        }
        let undefined = c.registers.filter { !$0.definedInFork }.count
        if undefined != c.undefinedInForkCount {
            return "undefined register count does not match the list"
        }
        return nil
    }

    public static func decodeAnalysis(_ data: Data) throws -> ResearchAnalysis {
        try decoder.decode(ResearchAnalysis.self, from: data)
    }

    public static func decodeMatrix(_ data: Data) throws -> ResearchMatrix {
        try decoder.decode(ResearchMatrix.self, from: data)
    }

    public static func decodeCensus(_ data: Data) throws -> ResearchCensus {
        try decoder.decode(ResearchCensus.self, from: data)
    }
}

/// Everything the research screens show, loaded once. Either all of it is valid or none is shown.
public struct ResearchCatalog: Equatable, Sendable {
    public let analysis: ResearchAnalysis
    public let matrix: ResearchMatrix
    public let census: ResearchCensus

    /// Decode and check the three files. Returns the first problem as a short English message.
    public static func build(analysis: Data, matrix: Data, census: Data) -> Result<ResearchCatalog, ResearchLoadError> {
        do {
            let a = try ResearchStore.decodeAnalysis(analysis)
            if let v = ResearchStore.violation(a) { return .failure(.invalid("analysis: \(v)")) }
            let m = try ResearchStore.decodeMatrix(matrix)
            if let v = ResearchStore.violation(m) { return .failure(.invalid("matrix: \(v)")) }
            let c = try ResearchStore.decodeCensus(census)
            if let v = ResearchStore.violation(c) { return .failure(.invalid("census: \(v)")) }
            return .success(ResearchCatalog(analysis: a, matrix: m, census: c))
        } catch {
            return .failure(.unreadable(String(describing: error)))
        }
    }
}

public enum ResearchLoadError: Error, Equatable, Sendable {
    case unreadable(String)
    case invalid(String)
    case missing(String)
}

extension ResearchCatalog {
    /// Load the three bundled files. A missing file is an error state, not an empty result.
    public static func loadFromBundle(_ bundle: Bundle = .main) -> Result<ResearchCatalog, ResearchLoadError> {
        func data(_ name: String) -> Result<Data, ResearchLoadError> {
            guard let url = bundle.url(forResource: name, withExtension: "json"),
                  let d = try? Data(contentsOf: url) else {
                return .failure(.missing(name))
            }
            return .success(d)
        }
        switch (data("t8130-firmware-analysis"), data("t8130-feasibility"), data("t8130-sptm-census")) {
        case let (.success(a), .success(m), .success(c)):
            return build(analysis: a, matrix: m, census: c)
        case let (.failure(e), _, _), let (_, .failure(e), _), let (_, _, .failure(e)):
            return .failure(e)
        }
    }

    /// Count of matrix components per status, in the order the screen shows them.
    public var statusCounts: [(ResearchStatus, Int)] {
        ResearchStatus.allCases.map { s in (s, matrix.components.filter { $0.status == s }.count) }
    }
}
