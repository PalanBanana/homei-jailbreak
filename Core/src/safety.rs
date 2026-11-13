//! Safety Manager.
//!
//! This release has no privileged backend, so the manager is a closed allow-list: only a short
//! set of read-only, in-app operations is ever allowed. Every other operation, including any name
//! it does not recognise, is denied with an explicit reason. There is no override and no hidden
//! execution path. Nothing here returns a fake success.

use serde::Serialize;

/// Operations the application can be asked to perform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    // Read-only, in-app operations.
    ViewCompatibility,
    ValidateData,
    ExportSanitizedDiagnostics,
    BrowseCatalog,
    SimulateBootstrapPlan,
    ViewRecoveryInfo,
    // Privileged or device-affecting operations. All unavailable.
    InstallJailbreak,
    ExecuteExploit,
    EscalatePrivileges,
    ModifyKernelMemory,
    BypassSptm,
    BypassTxm,
    BypassAmfi,
    ExtractProtectedMemory,
    ModifyRootFilesystem,
    InstallBootstrap,
    InjectTweak,
    InstallIpa,
    InstallPackage,
    ModifyTrustCache,
    RebootDevice,
    EnterDfuOrRecovery,
    UpdateFirmware,
    RestoreFirmware,
    DowngradeFirmware,
    ModifyOtaSettings,
    ModifyConfigurationProfile,
    ConnectUsbDevice,
    PairDevice,
    CreateBackup,
    CollectSysdiagnose,
    RunRemoteScript,
    /// Any name that is not recognised.
    Unknown(String),
}

impl Operation {
    /// Parse a snake_case operation name. Unrecognised names become `Unknown` and are denied.
    pub fn parse(name: &str) -> Operation {
        match name {
            "view_compatibility" => Operation::ViewCompatibility,
            "validate_data" => Operation::ValidateData,
            "export_sanitized_diagnostics" => Operation::ExportSanitizedDiagnostics,
            "browse_catalog" => Operation::BrowseCatalog,
            "simulate_bootstrap_plan" => Operation::SimulateBootstrapPlan,
            "view_recovery_info" => Operation::ViewRecoveryInfo,
            "install_jailbreak" => Operation::InstallJailbreak,
            "execute_exploit" => Operation::ExecuteExploit,
            "escalate_privileges" => Operation::EscalatePrivileges,
            "modify_kernel_memory" => Operation::ModifyKernelMemory,
            "bypass_sptm" => Operation::BypassSptm,
            "bypass_txm" => Operation::BypassTxm,
            "bypass_amfi" => Operation::BypassAmfi,
            "extract_protected_memory" => Operation::ExtractProtectedMemory,
            "modify_root_filesystem" => Operation::ModifyRootFilesystem,
            "install_bootstrap" => Operation::InstallBootstrap,
            "inject_tweak" => Operation::InjectTweak,
            "install_ipa" => Operation::InstallIpa,
            "install_package" => Operation::InstallPackage,
            "modify_trust_cache" => Operation::ModifyTrustCache,
            "reboot_device" => Operation::RebootDevice,
            "enter_dfu_or_recovery" => Operation::EnterDfuOrRecovery,
            "update_firmware" => Operation::UpdateFirmware,
            "restore_firmware" => Operation::RestoreFirmware,
            "downgrade_firmware" => Operation::DowngradeFirmware,
            "modify_ota_settings" => Operation::ModifyOtaSettings,
            "modify_configuration_profile" => Operation::ModifyConfigurationProfile,
            "connect_usb_device" => Operation::ConnectUsbDevice,
            "pair_device" => Operation::PairDevice,
            "create_backup" => Operation::CreateBackup,
            "collect_sysdiagnose" => Operation::CollectSysdiagnose,
            "run_remote_script" => Operation::RunRemoteScript,
            other => Operation::Unknown(other.to_string()),
        }
    }

    fn is_read_only_in_app(&self) -> bool {
        matches!(
            self,
            Operation::ViewCompatibility
                | Operation::ValidateData
                | Operation::ExportSanitizedDiagnostics
                | Operation::BrowseCatalog
                | Operation::SimulateBootstrapPlan
                | Operation::ViewRecoveryInfo
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum SafetyDecision {
    Allowed,
    Denied { reason: String },
}

impl SafetyDecision {
    pub fn is_allowed(&self) -> bool {
        matches!(self, SafetyDecision::Allowed)
    }
}

/// Authorise an operation. Only read-only, in-app operations pass.
pub fn authorize(op: &Operation) -> SafetyDecision {
    if op.is_read_only_in_app() {
        return SafetyDecision::Allowed;
    }
    let reason = match op {
        Operation::Unknown(name) => format!("Unrecognised operation '{name}' is denied by default."),
        Operation::ConnectUsbDevice | Operation::PairDevice => {
            "HOMEI has no device connection or pairing code. Any future device identification needs a separate, explicit authorisation outside this application.".to_string()
        }
        Operation::RunRemoteScript => {
            "Remote scripts are never executed, and metadata cannot carry commands.".to_string()
        }
        _ => "Privileged and device-affecting operations are unavailable: this research-stage build contains no verified backend.".to_string(),
    };
    SafetyDecision::Denied { reason }
}

/// Where an external artifact came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactOrigin {
    /// Bundled with the application and covered by the repository's checks.
    BundledFixture,
    /// A binary, package, profile or firmware file from anywhere else.
    ExternalUnverified,
}

/// Gate for external artifacts: binaries, packages, configuration profiles, firmware images.
pub fn authorize_artifact(origin: ArtifactOrigin) -> SafetyDecision {
    match origin {
        ArtifactOrigin::BundledFixture => SafetyDecision::Allowed,
        ArtifactOrigin::ExternalUnverified => SafetyDecision::Denied {
            reason: "External binaries, package sources, profiles and firmware artifacts are rejected until verified. This build can only read metadata.".to_string(),
        },
    }
}

/// Gate for compatibility. A profile can only be acted on when compatibility is positive, and
/// this build has no action to take, so the answer for non-positive states is a denial with the
/// state named.
pub fn require_verified_compatibility(status_label: &str, positive: bool) -> SafetyDecision {
    if positive {
        SafetyDecision::Allowed
    } else {
        SafetyDecision::Denied {
            reason: format!("Compatibility is {status_label}. Unverified compatibility never unlocks an operation."),
        }
    }
}

/// Result of invoking a privileged capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Unsupported {
    pub capability: &'static str,
    pub reason: &'static str,
}

/// A privileged capability and its status. Every entry is unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Capability {
    pub name: &'static str,
    pub state: &'static str,
    pub reason: &'static str,
}

const NO_BACKEND: &str =
    "No verified backend exists. This release contains no exploit implementation.";

/// Status of every privileged capability the product could one day expose.
pub fn privileged_capabilities() -> Vec<Capability> {
    [
        "Sandbox escape",
        "Container access",
        "Cross-process access",
        "Kernel read",
        "Kernel write",
        "Privilege escalation",
        "SPTM compatibility",
        "SPTM bypass",
        "AMFI compatibility",
        "Bootstrap",
        "Persistence",
        "Complete jailbreak",
    ]
    .into_iter()
    .map(|name| Capability {
        name,
        state: "unavailable",
        reason: NO_BACKEND,
    })
    .collect()
}

/// Invoke a privileged capability. Always an explicit unsupported result, never a success.
pub fn invoke_capability(name: &'static str) -> Result<(), Unsupported> {
    Err(Unsupported {
        capability: name,
        reason: NO_BACKEND,
    })
}
