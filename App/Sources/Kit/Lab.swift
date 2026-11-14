import Foundation

// Virtual research lab models. They mirror `Core/src/{target,fidelity,experiment,labstate}.rs`.
//
// The app never runs a virtual machine, Ghidra or a package manager. It only reads the sanitized
// `lab-state.json` that the host controller exports, and it decodes that file strictly: a key that
// the models do not know is an error, exactly as `deny_unknown_fields` is in Rust.

// MARK: Vocabularies

/// Whether a virtual machine for the target exists and what is known about it.
public enum LabVmStatus: String, Codable, CaseIterable, Sendable {
    case exactTargetVerified = "EXACT_TARGET_VERIFIED"
    case targetKernelBootedHardwareUnverified = "TARGET_KERNEL_BOOTED_HARDWARE_UNVERIFIED"
    case partialDarwinGuest = "PARTIAL_DARWIN_GUEST"
    case referenceDeviceOnly = "REFERENCE_DEVICE_ONLY"
    case unsupportedTarget = "UNSUPPORTED_TARGET"
    case blockedByMissingFirmware = "BLOCKED_BY_MISSING_FIRMWARE"
    case blockedByMissingQemuFeature = "BLOCKED_BY_MISSING_QEMU_FEATURE"
    case bootFailure = "BOOT_FAILURE"
    case notTested = "NOT_TESTED"

    /// True only for the single status that claims an exact target virtual machine.
    public var claimsExactTarget: Bool { self == .exactTargetVerified }
    public var localizationKey: String { "lab.vm_status.\(rawValue)" }
    public var detailLocalizationKey: String { "lab.vm_status.\(rawValue).detail" }
}

public enum LabHardwareFidelity: String, Codable, CaseIterable, Sendable {
    case exact = "EXACT", partial = "PARTIAL", approximate = "APPROXIMATE"
    case unsupported = "UNSUPPORTED", unverified = "UNVERIFIED"
    public var localizationKey: String { "lab.fidelity.\(rawValue)" }
}

public enum LabFidelityStatus: String, Codable, CaseIterable, Sendable {
    case exact = "EXACT", partial = "PARTIAL", approximate = "APPROXIMATE"
    case unsupported = "UNSUPPORTED", unverified = "UNVERIFIED", notTested = "NOT_TESTED"
    public var localizationKey: String { "lab.fidelity.\(rawValue)" }
}

public enum LabFirmwareIdentity: String, Codable, CaseIterable, Sendable {
    case exactReferenceMatch = "EXACT_REFERENCE_MATCH"
    case buildMismatch = "BUILD_MISMATCH"
    case modelMismatch = "MODEL_MISMATCH"
    case notEstablished = "NOT_ESTABLISHED"
    public var localizationKey: String { "lab.fw_identity.\(rawValue)" }
}

public enum LabBootOutcome: String, Codable, CaseIterable, Sendable {
    case notAttempted = "not_attempted", booted, timeout
    case kernelPanic = "kernel_panic", interrupted
    case processFailed = "process_failed", incompleteOutput = "incomplete_output"
    public var localizationKey: String { "lab.boot_outcome.\(rawValue)" }
}

public enum LabExperimentStatus: String, Codable, CaseIterable, Sendable {
    case planned = "PLANNED", running = "RUNNING", completed = "COMPLETED", failed = "FAILED"
    case interrupted = "INTERRUPTED", inconclusive = "INCONCLUSIVE", notReproducible = "NOT_REPRODUCIBLE"
    public var localizationKey: String { "lab.experiment_status.\(rawValue)" }
    /// An interrupted, failed or inconclusive experiment is never a success.
    public var isSuccess: Bool { self == .completed }
}

/// A mock is a test double and never counts as a real boot. `none` is a host-side operation.
public enum LabBackendKind: String, Codable, CaseIterable, Sendable {
    case hostOnly = "none", mock, real
    public var localizationKey: String { "lab.backend_kind.\(rawValue)" }
}

public enum LabSourceKind: String, Codable, Sendable { case primary, secondary, reference }
public enum LabConfidence: String, Codable, Sendable {
    case verified, partiallyVerified = "partially_verified", unverified
}

/// Guest root is never physical-device root.
public enum LabPrivilegeScope: String, Sendable {
    case guestRoot = "GUEST_ROOT", physicalDeviceRoot = "PHYSICAL_DEVICE_ROOT"
    public func implies(_ other: LabPrivilegeScope) -> Bool { self == other }
}

// MARK: Records

public struct LabFieldProvenance: Codable, Equatable, Sendable {
    public var source: String
    public var kind: LabSourceKind
    public var confidence: LabConfidence
    public var trackerClaimIds: [String]
    public var note: String
    enum CodingKeys: String, CodingKey {
        case source, kind, confidence, note
        case trackerClaimIds = "tracker_claim_ids"
    }
}

public struct LabDiscrepancy: Codable, Equatable, Identifiable, Sendable {
    public var id: String
    public var summary: String
    public var trackerClaimIds: [String]
    enum CodingKeys: String, CodingKey {
        case id, summary
        case trackerClaimIds = "tracker_claim_ids"
    }
}

public struct LabTarget: Codable, Equatable, Sendable {
    public var schemaVersion: Int
    public var targetId: String
    public var commercialModel: String
    public var productType: String
    public var soc: String
    public var cpu: String
    public var board: String
    public var cpid: String
    public var bdid: String
    public var architecture: String
    public var osVersion: String
    public var buildVersion: String
    public var vmStatus: LabVmStatus
    public var hardwareFidelity: LabHardwareFidelity
    public var provenance: [String: LabFieldProvenance]
    public var discrepancies: [LabDiscrepancy]
    public var checkedOn: String
    enum CodingKeys: String, CodingKey {
        case soc, cpu, board, cpid, bdid, architecture, provenance, discrepancies
        case schemaVersion = "schema_version", targetId = "target_id", commercialModel = "commercial_model"
        case productType = "product_type", osVersion = "os_version", buildVersion = "build_version"
        case vmStatus = "vm_status", hardwareFidelity = "hardware_fidelity", checkedOn = "checked_on"
    }

    /// The fixed research target. Another model, chip, board or build is never substituted.
    public static let canonical: [(field: String, value: String)] = [
        ("commercial_model", "iPhone 15 Pro Max"), ("product_type", "iPhone16,2"), ("soc", "Apple A17 Pro"),
        ("cpu", "t8130"), ("board", "D84AP"), ("cpid", "0x8130"), ("bdid", "0x06"),
        ("architecture", "arm64e"), ("os_version", "27.0.1"), ("build_version", "24A446"),
    ]

    /// Fields that must each carry provenance.
    public static let provenanceFields = canonical.map { $0.field }

    /// First reason the manifest is not the canonical target, or nil. Mirrors `TargetManifest::validate`.
    public func canonicalViolation() -> String? {
        if schemaVersion != 1 { return "unsupported schema version \(schemaVersion)" }
        let values: [String: String] = [
            "commercial_model": commercialModel, "product_type": productType, "soc": soc, "cpu": cpu,
            "board": board, "cpid": cpid, "bdid": bdid, "architecture": architecture,
            "os_version": osVersion, "build_version": buildVersion,
        ]
        for (field, expected) in Self.canonical where values[field] != expected {
            return "\(field) is \(values[field] ?? "?"), but the target is fixed to \(expected)"
        }
        for field in Self.provenanceFields {
            guard let p = provenance[field] else { return "field \(field) has no provenance" }
            if p.source.trimmingCharacters(in: .whitespaces).isEmpty { return "field \(field) has an empty provenance source" }
        }
        // Reference data describes the target. It cannot say a VM booted.
        if vmStatus != .notTested && vmStatus != .referenceDeviceOnly {
            return "reference metadata may not carry VM status \(vmStatus.rawValue)"
        }
        return nil
    }
}

public struct LabHost: Codable, Equatable, Sendable {
    public var distribution: String
    public var architecture: String
    public var qemuVersion: String?
    public var qemuAccelerators: [String]
    public var kvmDevicePresent: Bool
    public var ghidraVersion: String?
    public var memoryTotalGib: Int
    public var memoryAvailableGib: Int
    enum CodingKeys: String, CodingKey {
        case distribution, architecture
        case qemuVersion = "qemu_version", qemuAccelerators = "qemu_accelerators"
        case kvmDevicePresent = "kvm_device_present", ghidraVersion = "ghidra_version"
        case memoryTotalGib = "memory_total_gib", memoryAvailableGib = "memory_available_gib"
    }
}

public struct LabBackend: Codable, Equatable, Identifiable, Sendable {
    public var name: String
    public var available: Bool
    public var optional: Bool
    public var vmStatus: LabVmStatus
    public var blockers: [String]
    public var upstreamCommit: String?
    public var id: String { name }
    enum CodingKeys: String, CodingKey {
        case name, available, optional, blockers
        case vmStatus = "vm_status", upstreamCommit = "upstream_commit"
    }
}

public struct LabFirmwareArtifact: Codable, Equatable, Identifiable, Sendable {
    public var name: String
    public var bytes: UInt64
    public var sha256: String
    public var format: String
    public var sourceBuild: String
    public var role: String
    public var id: String { "\(name)#\(sha256)" }
    enum CodingKeys: String, CodingKey {
        case name, bytes, sha256, format, role
        case sourceBuild = "source_build"
    }
}

public struct LabGhidra: Codable, Equatable, Sendable {
    public var ghidraVersion: String?
    public var projectName: String
    public var projectPresent: Bool
    public var program: String
    public var firmwareSha256: String?
    public var architecture: String
    public var importStatus: String
    public var functionCount: UInt64
    public var filesetEntries: UInt64
    public var filesetEntriesCovered: UInt64
    public var symbolMatchBasisPoints: Int?
    public var importErrors: [String]
    public var lastAnalysisDate: String?
    public var sourceReference: String
    public var confidence: String
    enum CodingKeys: String, CodingKey {
        case program, architecture, confidence
        case ghidraVersion = "ghidra_version", projectName = "project_name", projectPresent = "project_present"
        case firmwareSha256 = "firmware_sha256", importStatus = "import_status", functionCount = "function_count"
        case filesetEntries = "fileset_entries", filesetEntriesCovered = "fileset_entries_covered"
        case symbolMatchBasisPoints = "symbol_match_basis_points", importErrors = "import_errors"
        case lastAnalysisDate = "last_analysis_date", sourceReference = "source_reference"
    }

    /// Symbol match rate as text, from integer basis points. No floating-point comparison is made.
    public var symbolMatchText: String? {
        guard let bp = symbolMatchBasisPoints else { return nil }
        return "\(bp / 100).\(String(format: "%02d", bp % 100))%"
    }
}

public struct LabPackage: Codable, Equatable, Identifiable, Sendable {
    public var id: String
    public var version: String
    public var provenance: String
    public var signature: String
    public var files: Int
}

public struct LabGuest: Codable, Equatable, Sendable {
    public var filesystemScope: String
    public var overlayPresent: Bool
    public var baseImageSha256: String?
    public var snapshots: Int
    public var vmRunning: Bool
    public var bootstrapVersion: String?
    public var bootstrapState: String
    public var packages: [LabPackage]
    public var transactions: Int
    enum CodingKeys: String, CodingKey {
        case snapshots, packages, transactions
        case filesystemScope = "filesystem_scope", overlayPresent = "overlay_present"
        case baseImageSha256 = "base_image_sha256", vmRunning = "vm_running"
        case bootstrapVersion = "bootstrap_version", bootstrapState = "bootstrap_state"
    }
}

public struct LabFidelity: Codable, Equatable, Sendable {
    public var targetId: String
    public var firmwareIdentity: LabFirmwareIdentity
    /// A. Does the analysis refer to the exact OS build?
    public var firmware: LabFidelityStatus
    /// B. Does the virtual machine reproduce the intended hardware target?
    public var hardware: LabFidelityStatus
    /// C. Which features are present in the guest, one entry per feature.
    public var functional: [String: LabFidelityStatus]
    public var vmStatus: LabVmStatus
    public var hardwareFidelity: LabHardwareFidelity
    public var notes: [String]
    enum CodingKeys: String, CodingKey {
        case firmware, hardware, functional, notes
        case targetId = "target_id", firmwareIdentity = "firmware_identity"
        case vmStatus = "vm_status", hardwareFidelity = "hardware_fidelity"
    }
}

public struct LabTargetRef: Codable, Equatable, Sendable {
    public var targetId: String
    public var productType: String
    public var soc: String
    public var board: String
    public var buildVersion: String
    enum CodingKeys: String, CodingKey {
        case soc, board
        case targetId = "target_id", productType = "product_type", buildVersion = "build_version"
    }
}

public struct LabBackendRef: Codable, Equatable, Sendable {
    public var name: String
    public var kind: LabBackendKind
    public var version: String?
    public var qemuVersion: String?
    enum CodingKeys: String, CodingKey {
        case name, kind, version
        case qemuVersion = "qemu_version"
    }
}

public struct LabArtifactHash: Codable, Equatable, Sendable {
    public var name: String
    public var sha256: String
}

public struct LabExperiment: Codable, Equatable, Identifiable, Sendable {
    public var experimentId: String
    public var kind: String
    public var target: LabTargetRef
    public var backend: LabBackendRef
    public var guestOsBuild: String?
    public var firmwareArtifacts: [LabArtifactHash]
    public var configSha256: String
    public var snapshotId: String?
    public var inputs: [String]
    public var timestamp: String
    public var status: LabExperimentStatus
    public var bootOutcome: LabBootOutcome?
    public var result: String
    public var logRefs: [LabArtifactHash]
    public var limitations: [String]
    public var reproducible: Bool?
    public var id: String { experimentId }
    enum CodingKeys: String, CodingKey {
        case kind, target, backend, inputs, timestamp, status, result, limitations, reproducible
        case experimentId = "experiment_id", guestOsBuild = "guest_os_build"
        case firmwareArtifacts = "firmware_artifacts", configSha256 = "config_sha256"
        case snapshotId = "snapshot_id", bootOutcome = "boot_outcome", logRefs = "log_refs"
    }
}

public enum LabEvidenceKind: String, Codable, Sendable {
    case staticObservation = "static_observation"
    case decompiledInterpretation = "decompiled_interpretation"
    case emulatorObservation = "emulator_observation"
    case reproducedBehavior = "reproduced_behavior"
    case hypothesis, unknown
}

public struct LabCorrelation: Codable, Equatable, Sendable {
    public var analysisProject: String
    public var firmwareSha256: String
    public var buildVersion: String
    public var experimentId: String
    public var guestIdentity: String
    public var configSha256: String
    public var evidenceKind: LabEvidenceKind
    public var evidenceSource: String
    public var confidence: String
    public var limitations: [String]
    enum CodingKeys: String, CodingKey {
        case confidence, limitations
        case analysisProject = "analysis_project", firmwareSha256 = "firmware_sha256"
        case buildVersion = "build_version", experimentId = "experiment_id", guestIdentity = "guest_identity"
        case configSha256 = "config_sha256", evidenceKind = "evidence_kind", evidenceSource = "evidence_source"
    }
}

public struct LabEvidenceItem: Codable, Equatable, Identifiable, Sendable {
    public var id: String
    public var category: String
    public var title: String
    public var reference: String
    public var limitation: String
}

public struct LabState: Codable, Equatable, Sendable {
    public var schemaVersion: Int
    public var generatedOn: String
    public var target: LabTarget
    public var host: LabHost
    public var backends: [LabBackend]
    public var firmware: [LabFirmwareArtifact]
    public var ghidra: LabGhidra
    public var guest: LabGuest
    public var fidelity: LabFidelity
    public var experiments: [LabExperiment]
    public var correlations: [LabCorrelation]
    public var evidence: [LabEvidenceItem]
    public var unresolved: [String]
    enum CodingKeys: String, CodingKey {
        case target, host, backends, firmware, ghidra, guest, fidelity
        case experiments, correlations, evidence, unresolved
        case schemaVersion = "schema_version", generatedOn = "generated_on"
    }
}

// MARK: Strict decoding and validation

public enum LabStateError: Error, CustomStringConvertible, Equatable {
    case missing(String)
    case unknownFields([String])
    case invalid(String)
    public var description: String {
        switch self {
        case .missing(let f): return "Missing bundled file \(f)"
        case .unknownFields(let keys): return "unknown fields: \(keys.joined(separator: ", "))"
        case .invalid(let why): return "invalid lab state: \(why)"
        }
    }
}

public enum LabStore {
    /// Decode and validate. A key the models do not know is rejected, as `deny_unknown_fields` does in Rust.
    public static func decode(_ data: Data) throws -> LabState {
        let state: LabState
        do { state = try JSONDecoder().decode(LabState.self, from: data) }
        catch { throw LabStateError.invalid("\(error)") }
        let original = try JSONSerialization.jsonObject(with: data)
        let known = try JSONSerialization.jsonObject(with: JSONEncoder().encode(state))
        let extra = extraKeys(original, known, path: "")
        if !extra.isEmpty { throw LabStateError.unknownFields(extra) }
        if let why = violation(state) { throw LabStateError.invalid(why) }
        return state
    }

    /// Load `lab-state.json` from an app bundle.
    public static func bundled(_ bundle: Bundle = .main) throws -> LabState {
        guard let url = bundle.url(forResource: "lab-state", withExtension: "json") else {
            throw LabStateError.missing("lab-state.json")
        }
        return try decode(Data(contentsOf: url))
    }

    /// Keys present in the original document that the re-encoded model does not have. A `null` for an
    /// optional field is not a key the model lacks.
    static func extraKeys(_ original: Any, _ known: Any, path: String) -> [String] {
        if let o = original as? [String: Any] {
            let k = known as? [String: Any] ?? [:]
            var out: [String] = []
            for (key, value) in o.sorted(by: { $0.key < $1.key }) {
                if value is NSNull { continue }
                guard let kv = k[key] else { out.append("\(path)/\(key)"); continue }
                out += extraKeys(value, kv, path: "\(path)/\(key)")
            }
            return out
        }
        if let o = original as? [Any], let k = known as? [Any] {
            var out: [String] = []
            for (index, value) in o.enumerated() where index < k.count {
                out += extraKeys(value, k[index], path: "\(path)[\(index)]")
            }
            return out
        }
        return []
    }

    /// Mirrors `LabState::validate` in Rust: the canonical target, a fidelity report for the same
    /// target, and no text that looks like a local path.
    public static func violation(_ state: LabState) -> String? {
        if let why = state.target.canonicalViolation() { return why }
        if state.fidelity.targetId != state.target.targetId { return "fidelity report is for a different target" }
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.withoutEscapingSlashes]
        if let data = try? encoder.encode(state), let json = String(data: data, encoding: .utf8) {
            // JSON escapes the backslash, so the Windows marker is checked in its escaped form too.
            for marker in ["/home/", "/Users/", "/tmp/", "C:\\\\", "C:\\"] where json.contains(marker) {
                return "the snapshot contains a local path marker \(marker)"
            }
        }
        return nil
    }
}

// MARK: Presentation helpers

public extension LabState {
    /// The feature names the fidelity report always lists, in display order.
    static let featureOrder = ["guest_boot", "serial_console", "springboard", "graphics", "wifi", "bluetooth", "kernel_research"]

    /// True if any experiment came from a real backend. A host workspace operation is not a boot.
    var hasRealBackendExperiment: Bool { experiments.contains { $0.backend.kind == .real } }

    /// The one-line honest summary of what the lab can claim.
    var claimsExactTargetVM: Bool { fidelity.vmStatus.claimsExactTarget }
}
