import Foundation

/// Where a profile came from. No provenance means "connected device".
public enum Provenance: String, Codable, CaseIterable, Sendable {
    case manualProfile = "manual_profile"
    case testFixture = "test_fixture"
    case userAuthorizedReadOnlyObservation = "user_authorized_read_only_observation"
    case publicFirmwareMetadata = "public_firmware_metadata"

    public var label: String {
        switch self {
        case .manualProfile: return "Manual profile"
        case .testFixture: return "Test fixture (not a connected device)"
        case .userAuthorizedReadOnlyObservation: return "User-authorized read-only observation (imported, not live)"
        case .publicFirmwareMetadata: return "Public firmware metadata"
        }
    }
}

public enum SocFamily: String, Sendable {
    case a17Pro = "A17 Pro", a18Pro = "A18 Pro", a19Pro = "A19 Pro"

    /// Sourced product-type mapping only. Partially verified in the research tracker.
    public static func from(productType: String) -> SocFamily? {
        switch productType {
        case "iPhone16,2": return .a17Pro
        case "iPhone17,2": return .a18Pro
        case "iPhone18,1": return .a19Pro
        default: return nil
        }
    }
}

public struct ProfileError: Error, Equatable, CustomStringConvertible, Sendable {
    public let field: String
    public let reason: String
    public var description: String { "invalid \(field): \(reason)" }
}

private struct AnyKey: CodingKey {
    var stringValue: String
    var intValue: Int? { nil }
    init?(stringValue: String) { self.stringValue = stringValue }
    init?(intValue: Int) { return nil }
}

/// The six permitted metadata fields plus provenance. Unknown keys are rejected on decode so
/// identifiers cannot be imported as extra fields.
public struct DeviceProfile: Codable, Equatable, Sendable {
    public var productType: String
    public var productVersion: String
    public var buildVersion: String
    public var deviceClass: String?
    public var hardwareModel: String?
    public var cpuArchitecture: String?
    public var provenance: Provenance

    enum CodingKeys: String, CodingKey, CaseIterable {
        case productType = "product_type", productVersion = "product_version", buildVersion = "build_version"
        case deviceClass = "device_class", hardwareModel = "hardware_model", cpuArchitecture = "cpu_architecture"
        case provenance
    }

    public init(productType: String, productVersion: String, buildVersion: String, deviceClass: String? = nil,
                hardwareModel: String? = nil, cpuArchitecture: String? = nil, provenance: Provenance) {
        self.productType = productType; self.productVersion = productVersion; self.buildVersion = buildVersion
        self.deviceClass = deviceClass; self.hardwareModel = hardwareModel
        self.cpuArchitecture = cpuArchitecture; self.provenance = provenance
    }

    public init(from decoder: Decoder) throws {
        let any = try decoder.container(keyedBy: AnyKey.self)
        let allowed = Set(CodingKeys.allCases.map { $0.rawValue })
        for key in any.allKeys where !allowed.contains(key.stringValue) {
            throw DecodingError.dataCorruptedError(forKey: key, in: any, debugDescription: "unknown field \(key.stringValue)")
        }
        let c = try decoder.container(keyedBy: CodingKeys.self)
        productType = try c.decode(String.self, forKey: .productType)
        productVersion = try c.decode(String.self, forKey: .productVersion)
        buildVersion = try c.decode(String.self, forKey: .buildVersion)
        deviceClass = try c.decodeIfPresent(String.self, forKey: .deviceClass)
        hardwareModel = try c.decodeIfPresent(String.self, forKey: .hardwareModel)
        cpuArchitecture = try c.decodeIfPresent(String.self, forKey: .cpuArchitecture)
        provenance = try c.decode(Provenance.self, forKey: .provenance)
    }

    public var socFamily: SocFamily? { SocFamily.from(productType: productType) }

    public func validate() throws {
        try Self.require(productType, "product_type", #"^[A-Za-z]+[0-9]{1,3},[0-9]{1,3}$"#)
        try Self.require(productVersion, "product_version", #"^[0-9]{1,3}(\.[0-9]{1,3}){1,2}$"#)
        try Self.require(buildVersion, "build_version", #"^[0-9]{1,3}[A-Z][0-9]{1,5}[a-z]?$"#)
        for (name, value) in [("device_class", deviceClass), ("hardware_model", hardwareModel), ("cpu_architecture", cpuArchitecture)] {
            if let v = value {
                try Self.require(v, name, #"^[A-Za-z0-9,._-]{1,32}$"#)
                if looksLikeIdentifier(v) { throw ProfileError(field: name, reason: "identifier-like value") }
            }
        }
    }

    private static func require(_ s: String, _ field: String, _ pattern: String) throws {
        if s.range(of: pattern, options: .regularExpression) == nil || looksLikeIdentifier(s) {
            throw ProfileError(field: field, reason: "unexpected shape or identifier-like value")
        }
    }
}

/// Heuristic for values that look like device or account identifiers. Errs toward rejecting.
public func looksLikeIdentifier(_ s: String) -> Bool {
    let hex = Set("0123456789abcdefABCDEF")
    let only40Hex = s.count == 40 && s.allSatisfy { hex.contains($0) }
    let chars = Array(s)
    let udidShape = chars.count == 25 && chars[8] == "-" && chars.filter { $0 != "-" }.allSatisfy { hex.contains($0) }
    var run = 0, longest = 0
    for ch in chars {
        if ch.isASCII && ch.isNumber { run += 1; longest = max(longest, run) } else { run = 0 }
    }
    return only40Hex || udidShape || longest >= 14 || chars.count > 32
}
