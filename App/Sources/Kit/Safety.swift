import Foundation

/// Closed allow-list. Only read-only, in-app operations pass; everything else, including any
/// unrecognised name, is denied with a reason. There is no override.
public enum SafetyManager {
    public static let readOnlyOperations: Set<String> = [
        "view_compatibility", "validate_data", "export_sanitized_diagnostics",
        "browse_catalog", "simulate_bootstrap_plan", "view_recovery_info",
    ]

    public enum Decision: Equatable, Sendable {
        case allowed
        case denied(reason: String)
        public var isAllowed: Bool { self == .allowed }
    }

    public static func authorize(operation name: String) -> Decision {
        if readOnlyOperations.contains(name) { return .allowed }
        switch name {
        case "connect_usb_device", "pair_device":
            return .denied(reason: "HOMEI has no device connection or pairing code. Any future device identification needs a separate, explicit authorisation outside this application.")
        case "run_remote_script":
            return .denied(reason: "Remote scripts are never executed, and metadata cannot carry commands.")
        default:
            return .denied(reason: "Privileged, device-affecting and unrecognised operations are unavailable: this research-stage build contains no verified backend.")
        }
    }

    public struct Capability: Equatable, Sendable { public let name: String; public let reason: String }

    /// Every privileged capability, all unavailable.
    public static let privilegedCapabilities: [Capability] = [
        "Sandbox escape", "Container access", "Cross-process access", "Kernel read", "Kernel write",
        "Privilege escalation", "SPTM compatibility", "SPTM bypass", "AMFI compatibility",
        "Bootstrap", "Persistence", "Complete jailbreak",
    ].map { Capability(name: $0, reason: "No verified backend exists. This release contains no exploit implementation.") }
}
