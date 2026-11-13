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

    /// Operations that are recognised and denied. Anything in neither set is "unrecognised".
    public static let knownPrivilegedOperations: Set<String> = [
        "install_jailbreak", "execute_exploit", "escalate_privileges", "modify_kernel_memory", "bypass_sptm",
        "bypass_txm", "bypass_amfi", "extract_protected_memory", "modify_root_filesystem", "install_bootstrap",
        "inject_tweak", "install_ipa", "install_package", "modify_trust_cache", "reboot_device",
        "enter_dfu_or_recovery", "update_firmware", "restore_firmware", "downgrade_firmware", "modify_ota_settings",
        "modify_configuration_profile", "connect_usb_device", "pair_device", "create_backup", "collect_sysdiagnose",
        "run_remote_script",
    ]

    /// Reason texts are identical to the Rust core so the two implementations can be compared exactly.
    public static func authorize(operation name: String) -> Decision {
        if readOnlyOperations.contains(name) { return .allowed }
        switch name {
        case "connect_usb_device", "pair_device":
            return .denied(reason: "HOMEI has no device connection or pairing code. Any future device identification needs a separate, explicit authorisation outside this application.")
        case "run_remote_script":
            return .denied(reason: "Remote scripts are never executed, and metadata cannot carry commands.")
        default:
            if knownPrivilegedOperations.contains(name) {
                return .denied(reason: "Privileged and device-affecting operations are unavailable: this research-stage build contains no verified backend.")
            }
            return .denied(reason: "Unrecognised operation '\(name)' is denied by default.")
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
