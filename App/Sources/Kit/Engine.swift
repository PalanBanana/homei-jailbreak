import Foundation

public enum CompatibilityStatus: String, Codable, CaseIterable, Sendable {
    case supported, unsupported
    case partiallySupported = "partially_supported"
    case notEstablished = "not_established"
    case unknown

    /// Key into Localizable.strings.
    public var localizationKey: String { "status.\(rawValue)" }

    public var label: String {
        switch self {
        case .supported: return "SUPPORTED"
        case .unsupported: return "UNSUPPORTED"
        case .partiallySupported: return "PARTIALLY SUPPORTED"
        case .notEstablished: return "NOT ESTABLISHED"
        case .unknown: return "UNKNOWN"
        }
    }

    public var isPositive: Bool { self == .supported || self == .partiallySupported }
}

public enum EvidenceClass: String, Codable, Sendable {
    case confirmedDeviceMetadata = "confirmed_device_metadata"
    case publicSourceStatement = "public_source_statement"
    case officialAppleDocumentation = "official_apple_documentation"
    case staticBinaryObservation = "static_binary_observation"
    case reportedRuntimeResult = "reported_runtime_result"
    case independentReproduction = "independent_reproduction"
    case targetCompatibleResult = "target_compatible_result"
    case unverified, unknown
}

public enum Confidence: String, Codable, Sendable { case high, medium, low }

public struct EvidenceScope: Codable, Equatable, Sendable {
    public var productType: String?
    public var productVersion: String?
    public var buildVersion: String?
    public var socFamily: String?
    enum CodingKeys: String, CodingKey {
        case productType = "product_type", productVersion = "product_version"
        case buildVersion = "build_version", socFamily = "soc_family"
    }
    public init(productType: String? = nil, productVersion: String? = nil, buildVersion: String? = nil, socFamily: String? = nil) {
        self.productType = productType; self.productVersion = productVersion
        self.buildVersion = buildVersion; self.socFamily = socFamily
    }
    func matchesExactly(_ p: DeviceProfile) -> Bool {
        productType == p.productType && productVersion == p.productVersion && buildVersion == p.buildVersion
    }
}

public struct Evidence: Codable, Equatable, Identifiable, Sendable {
    public var id: String
    public var trackerClaimId: String?
    public var title: String
    public var evidenceClass: EvidenceClass
    public var scope: EvidenceScope
    public var sourceUrls: [String]
    public var publicationDate: String?
    public var accessDate: String?
    public var confidence: Confidence
    public var limitations: String
    public var contradictingIds: [String]
    enum CodingKeys: String, CodingKey {
        case id, title, scope, confidence, limitations
        case trackerClaimId = "tracker_claim_id", evidenceClass = "class", sourceUrls = "source_urls"
        case publicationDate = "publication_date", accessDate = "access_date", contradictingIds = "contradicting_ids"
    }
    public init(id: String, trackerClaimId: String? = nil, title: String = "synthetic", evidenceClass: EvidenceClass,
                scope: EvidenceScope, sourceUrls: [String] = [], publicationDate: String? = nil, accessDate: String? = nil,
                confidence: Confidence, limitations: String = "", contradictingIds: [String] = []) {
        self.id = id; self.trackerClaimId = trackerClaimId; self.title = title; self.evidenceClass = evidenceClass
        self.scope = scope; self.sourceUrls = sourceUrls; self.publicationDate = publicationDate
        self.accessDate = accessDate; self.confidence = confidence; self.limitations = limitations
        self.contradictingIds = contradictingIds
    }
}

public struct Declaration: Codable, Equatable, Identifiable, Sendable {
    public var id: String
    public var productType: String
    public var socFamily: String
    public var productVersion: String
    public var buildVersion: String
    public var architecture: String?
    public var status: CompatibilityStatus
    public var evidenceIds: [String]
    public var note: String
    enum CodingKeys: String, CodingKey {
        case id, architecture, status, note
        case productType = "product_type", socFamily = "soc_family", productVersion = "product_version"
        case buildVersion = "build_version", evidenceIds = "evidence_ids"
    }
    public init(id: String, productType: String, socFamily: String, productVersion: String, buildVersion: String,
                architecture: String?, status: CompatibilityStatus, evidenceIds: [String], note: String) {
        self.id = id; self.productType = productType; self.socFamily = socFamily; self.productVersion = productVersion
        self.buildVersion = buildVersion; self.architecture = architecture; self.status = status
        self.evidenceIds = evidenceIds; self.note = note
    }
}

public struct Decision: Equatable, Sendable {
    public var status: CompatibilityStatus
    public var matchedDeclaration: String?
    public var downgraded: Bool
    public var provenance: Provenance
    public var explanation: [String]
}

/// Evidence-driven, deterministic matching. Mirrors the Rust core: exact device and build only;
/// a positive declaration without exact-scope evidence is reduced to `notEstablished`.
public enum CompatibilityEngine {
    public static func evaluate(profile: DeviceProfile, declarations: [Declaration], evidence: [Evidence]) -> Decision {
        func result(_ s: CompatibilityStatus, _ why: String, matched: String? = nil, downgraded: Bool = false) -> Decision {
            Decision(status: s, matchedDeclaration: matched, downgraded: downgraded, provenance: profile.provenance, explanation: [why])
        }
        do { try profile.validate() } catch {
            return result(.unknown, "Profile rejected, so nothing is evaluated: \(error).")
        }
        guard profile.socFamily != nil else {
            return result(.unknown, "No sourced SoC mapping exists for this product type.")
        }
        let sameTriple = declarations.filter {
            $0.productType == profile.productType && $0.productVersion == profile.productVersion && $0.buildVersion == profile.buildVersion
        }
        let exact = sameTriple.first { d in
            if let a = d.architecture, let b = profile.cpuArchitecture { return a == b }
            return true
        }
        guard let decl = exact else {
            if !sameTriple.isEmpty {
                return result(.notEstablished, "A record exists for this device and build, but its architecture differs from the profile.")
            }
            if declarations.contains(where: { $0.productType == profile.productType }) {
                return result(.notEstablished, "Records exist for \(profile.productType) but none for version \(profile.productVersion) build \(profile.buildVersion). Results for other versions or other devices do not transfer.")
            }
            return result(.unknown, "No record exists for this product type.")
        }
        func has(_ cls: EvidenceClass) -> Bool {
            decl.evidenceIds.compactMap { id in evidence.first { $0.id == id } }.contains {
                $0.evidenceClass == cls && $0.scope.matchesExactly(profile) && $0.confidence != .low && $0.contradictingIds.isEmpty
            }
        }
        let target = has(.targetCompatibleResult), repro = has(.independentReproduction)
        switch decl.status {
        case .supported where target && repro:
            return result(.supported, "Exact-scope target result and independent reproduction are both on record.", matched: decl.id)
        case .partiallySupported where target || repro:
            return result(.partiallySupported, "Exact-scope evidence is on record for part of the capability set.", matched: decl.id)
        case .supported, .partiallySupported:
            return result(.notEstablished, "Record \(decl.id) declares \(decl.status.label) but cites no exact-scope evidence of the required class, so the state is reduced.", matched: decl.id, downgraded: true)
        default:
            return result(decl.status, "Record \(decl.id): \(decl.note)", matched: decl.id)
        }
    }

    /// This build offers no jailbreak action.
    public static let jailbreakActionAvailable = false
    public static let researchOnlyLabel = "Research only. No verified jailbreak backend exists."
}
