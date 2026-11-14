//! T8130 feasibility matrix.
//!
//! The matrix answers, for each component of the target, whether the host backend can model it
//! and whether it is needed to boot. It reads the firmware analysis bundle for target evidence and
//! a fixed table of recorded research findings (each with its source file) for the backend and
//! the static hardware comparisons. It never infers compatibility from another SoC, and it never
//! claims an exact target VM. A component is `BLOCKED` or `UNSUPPORTED` unless its own evidence
//! says otherwise.

use crate::analysis::confidence::Confidence;
use crate::analysis::research::{Availability, Bundle};
use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;
pub const LAST_VERIFIED: &str = "2026-11-14";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Status {
    /// The target evidence establishes the property.
    Verified,
    /// Some of the property is established, some is not.
    Partial,
    /// A concrete blocker prevents the property. The blocker is named in the basis.
    Blocked,
    /// The current backend has no model for the property.
    Unsupported,
    /// No evidence either way.
    Unverified,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Status::Verified => "VERIFIED",
            Status::Partial => "PARTIAL",
            Status::Blocked => "BLOCKED",
            Status::Unsupported => "UNSUPPORTED",
            Status::Unverified => "UNVERIFIED",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decision {
    pub statement: String,
    pub source: String,
    pub evidence_type: String,
    pub confidence: Confidence,
    pub last_verified: String,
    pub limitation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeasibilityComponent {
    pub id: String,
    pub title: String,
    /// The question the status answers, stated so it can be checked.
    pub question: String,
    pub status: Status,
    pub basis: String,
    pub decisions: Vec<Decision>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactTargetVm {
    pub status: String,
    pub statement: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeasibilityMatrix {
    pub schema_version: u32,
    pub target: String,
    pub generated_on: String,
    pub analysis_analysed_on: String,
    pub exact_target_vm: ExactTargetVm,
    pub components: Vec<FeasibilityComponent>,
}

// Recorded research sources, relative to the research workspace.
const SRC_ANALYSIS: &str = "firmware analysis bundle (t8130-firmware-analysis.json)";
const SRC_AIC: &str = "research/hardware/t8130-aic-static-correlation.md";
const SRC_SPTM: &str =
    "research/hardware/sptm-register-census.md; research/hardware/cpreg-gap.json";
const SRC_BOOT: &str = "research/boot-chain/iphone16,2-handoff-analysis.md";
const SRC_DISPLAY: &str = "research/display/iphone16,2-display-pipeline.md";
const SRC_PROBE: &str = "logs/phase9-actual-boot-report.md";
const SRC_BLOCKERS: &str = "research/limitations/exact-blockers.md";

fn decision(
    statement: impl Into<String>,
    source: &str,
    evidence_type: &str,
    confidence: Confidence,
    limitation: impl Into<String>,
) -> Decision {
    Decision {
        statement: statement.into(),
        source: source.to_string(),
        evidence_type: evidence_type.to_string(),
        confidence,
        last_verified: LAST_VERIFIED.to_string(),
        limitation: limitation.into(),
    }
}

fn component(
    id: &str,
    title: &str,
    question: &str,
    status: Status,
    basis: &str,
    decisions: Vec<Decision>,
) -> FeasibilityComponent {
    FeasibilityComponent {
        id: id.to_string(),
        title: title.to_string(),
        question: question.to_string(),
        status,
        basis: basis.to_string(),
        decisions,
    }
}

/// The availability of a named manifest component, or an error when the manifest does not have it.
fn availability(bundle: &Bundle, name: &str) -> Result<Availability, String> {
    bundle
        .components
        .iter()
        .find(|c| c.name == name)
        .map(|c| c.availability)
        .ok_or_else(|| format!("manifest has no {name} component"))
}

fn availability_label(a: Availability) -> &'static str {
    match a {
        Availability::Analysed => "analysed",
        Availability::ContainerOnly => "a container only",
        Availability::Encrypted => "encrypted (keys not available)",
        Availability::Unsupported => "an unsupported format",
        Availability::Unavailable => "unavailable locally",
        Availability::Missing => "missing from the archive",
        Availability::NoManifestPath => "not located (no path in the manifest)",
    }
}

/// Refuse to evaluate a bundle that does not describe the fixed target.
fn check_bundle(bundle: &Bundle) -> Result<(), String> {
    let failed: Vec<&str> = bundle
        .identity_checks
        .iter()
        .filter(|c| !c.passed)
        .map(|c| c.id.as_str())
        .collect();
    if !failed.is_empty() {
        return Err(format!(
            "identity checks failed, refusing to evaluate: {}",
            failed.join(", ")
        ));
    }
    if bundle.components.is_empty() {
        return Err("analysis has no components".to_string());
    }
    Ok(())
}

/// Evaluate the matrix. Every status is stated by its component's own basis; nothing defaults.
pub fn evaluate(bundle: &Bundle, date: &str) -> Result<FeasibilityMatrix, String> {
    check_bundle(bundle)?;
    let dt = bundle
        .devicetree
        .as_ref()
        .ok_or("analysis has no device tree record")?;
    let kc = bundle
        .kernelcache
        .as_ref()
        .ok_or("analysis has no kernelcache record")?;
    let aic = dt.aic.as_ref().ok_or("device tree has no AIC node")?;
    let pmgr = dt.pmgr.as_ref().ok_or("device tree has no PMGR node")?;
    let sep = availability(bundle, "SEP")?;
    let sptm = availability(bundle, "Ap,SecurePageTableMonitor")?;
    let txm = availability(bundle, "Ap,TrustedExecutionMonitor")?;
    let gfx = availability(bundle, "GFX")?;
    let dcp = availability(bundle, "DCP")?;
    let ans = availability(bundle, "ANS")?;
    let system = availability(bundle, "OS")?;
    let ramdisk = availability(bundle, "RestoreRamDisk")?;

    let identity = decision(
        "BuildManifest identity matches iPhone16,2, D84AP, ApChipID 0x8130, build 24A446, 27.0.1",
        SRC_ANALYSIS,
        "static manifest",
        Confidence::Manifest,
        "Identity only. It does not show that any hardware model is available",
    );
    let qemu_absent = decision(
        "The installed QEMU (11.1.3) was probed with a generic virt machine and the max CPU only; no T8130 CPU model, darwin machine or SEP model was present",
        SRC_PROBE,
        "recorded backend probe",
        Confidence::Target,
        "Recorded in phase 9. Not re-run in this phase, per the static-only constraint",
    );

    let arm_io_parent = dt
        .arm_io_ranges
        .first()
        .map_or("not present".to_string(), |r| r.parent_base_hex.clone());
    let aic_translated = aic
        .translated_base_hex
        .clone()
        .unwrap_or_else(|| "not established".to_string());
    let timebase = dt
        .aic_timebase_reg
        .first()
        .map_or("not present".to_string(), |r| r.address_hex.clone());

    let components = vec![
        component(
            "cpu",
            "CPU T8130",
            "Can the backend model the T8130 CPU (arm64e, GXF/SPRR)?",
            Status::Unsupported,
            "Identity is verified. The installed backend has no T8130 CPU model",
            vec![identity.clone(), qemu_absent.clone()],
        ),
        component(
            "memory_map",
            "Memory map",
            "Is the target's physical memory layout modelled?",
            Status::Partial,
            "The AIC window translates through the first /arm-io ranges entry. DRAM and carve-outs are filled in at boot and are not in the static tree",
            vec![
                decision(
                    format!("The first /arm-io ranges entry maps child 0x0 to parent {arm_io_parent}"),
                    SRC_ANALYSIS,
                    "static device tree",
                    Confidence::Target,
                    "Read with the declared cell widths of the node",
                ),
                decision(
                    format!("The AIC base translates to {aic_translated}"),
                    SRC_AIC,
                    "static device tree arithmetic",
                    Confidence::Inferred,
                    "The arithmetic is established. That iBoot maps the window there is not",
                ),
            ],
        ),
        component(
            "interrupt_controller",
            "Interrupt controller (AIC, aic,3)",
            "Can an AIC model for aic,3 be bound to the target's interrupt tree?",
            Status::Blocked,
            "No model. Register semantics are not known: the kernel reads the offsets from the tree, and aic-iack-offset is named in iBoot but absent from the static tree",
            vec![
                decision(
                    format!(
                        "The AIC compatible is {} with {} interrupt cell(s)",
                        aic.compatible.join(", "),
                        aic.interrupt_cells.map_or("no".to_string(), |c| c.to_string())
                    ),
                    SRC_ANALYSIS,
                    "static device tree",
                    Confidence::Target,
                    "One cell is Apple's convention, confirmed by the kernel diagnostic string in the analysis",
                ),
                decision(
                    "The upstream Linux AIC bindings require three or four interrupt cells and do not list aic,3",
                    SRC_AIC,
                    "public source comparison",
                    Confidence::Inferred,
                    "Comparison with the public Linux tree, read only. It does not show what Apple's driver does",
                ),
                decision(
                    "The kernel has an AppleInterruptControllerV3 personality matched to aic,3, and the offset property names",
                    SRC_AIC,
                    "static kernelcache strings",
                    Confidence::Target,
                    "Strings only. No function cross-references were computed",
                ),
            ],
        ),
        component(
            "timer",
            "Timer (aic-timebase)",
            "Is the always-on timer modelled?",
            Status::Unverified,
            "A node exists at the translated window. Its register semantics and any backend model are unknown",
            vec![decision(
                format!("The aic-timebase node reg address is {timebase}"),
                SRC_ANALYSIS,
                "static device tree",
                Confidence::Target,
                "Node presence and address only. No timer behaviour was tested",
            )],
        ),
        component(
            "pmgr",
            "Power manager (PMGR, pmgr1,t8130)",
            "Can the PMGR be modelled, and its power domains bound?",
            Status::Blocked,
            "The compatible is not in the public Linux bindings, the tree has no per-domain nodes, and the blob layout is not documented",
            vec![
                decision(
                    format!(
                        "The PMGR compatible is {}, with {} register windows and {} power-domain names",
                        pmgr.compatible.join(", "),
                        pmgr.reg_windows,
                        pmgr.power_domain_names.len()
                    ),
                    SRC_ANALYSIS,
                    "static device tree",
                    Confidence::Target,
                    "Names are ASCII tokens from the blob. No record layout is assumed",
                ),
                decision(
                    "The kernel has an AppleT8130PMGR personality matched to pmgr1,t8130",
                    SRC_AIC,
                    "static kernelcache strings",
                    Confidence::Target,
                    "Strings only",
                ),
            ],
        ),
        component(
            "display",
            "Display controller (DCP)",
            "Can the display coprocessor be modelled, and does the backend show a target framebuffer?",
            Status::Unsupported,
            "The DCP firmware is present and unencrypted, but there is no DCP model. The only framebuffer is a generic placeholder",
            vec![
                decision(
                    format!("The DCP firmware component is {}", availability_label(dcp)),
                    SRC_ANALYSIS,
                    "static IM4P container",
                    Confidence::Target,
                    "Firmware for the real coprocessor. Not executed",
                ),
                decision(
                    "The recorded probe screenshot shows the generic QEMU placeholder, not an iOS display",
                    SRC_DISPLAY,
                    "recorded backend probe",
                    Confidence::Target,
                    "Generic machine. Not a target result",
                ),
            ],
        ),
        component(
            "gpu",
            "GPU (AGX)",
            "Can the GPU firmware and device be modelled?",
            Status::Unsupported,
            "GPU firmware is present and unencrypted. The installed backend has no GPU model for the target",
            vec![
                decision(
                    format!("The GFX firmware component is {}", availability_label(gfx)),
                    SRC_ANALYSIS,
                    "static IM4P container",
                    Confidence::Target,
                    "Firmware for the real GPU. Not executed",
                ),
                qemu_absent.clone(),
            ],
        ),
        component(
            "usb",
            "USB controllers",
            "Are the target USB controllers identified and modelled?",
            Status::Unverified,
            "Not researched in the static analysis. No USB model is recorded",
            vec![decision(
                "No USB controller analysis is recorded for the target",
                SRC_BLOCKERS,
                "recorded research note",
                Confidence::Unknown,
                "Nothing was established",
            )],
        ),
        component(
            "storage",
            "Storage (ANS, system volume)",
            "Can the storage coprocessor run, and is a full system volume available?",
            Status::Blocked,
            "The ANS firmware is analysed but has no model, and the system volume image is not available locally",
            vec![
                decision(
                    format!("The ANS firmware component is {}", availability_label(ans)),
                    SRC_ANALYSIS,
                    "static IM4P container",
                    Confidence::Target,
                    "Firmware for the real storage coprocessor. Not executed",
                ),
                decision(
                    format!("The system volume (OS) component is {}", availability_label(system)),
                    SRC_ANALYSIS,
                    "archive listing",
                    Confidence::Target,
                    "The archive lists it. There is no local copy, and its key service is external",
                ),
            ],
        ),
        component(
            "sep",
            "SEP interface",
            "Can the Secure Enclave firmware be run or emulated?",
            Status::Blocked,
            "The SEP firmware is encrypted with a keybag and its keys are not available. No SEP model exists for this target",
            vec![
                decision(
                    format!("The SEP firmware component is {}", availability_label(sep)),
                    SRC_ANALYSIS,
                    "static IM4P keybag",
                    Confidence::Target,
                    "The keys are not in the archive. No decryption was attempted",
                ),
                qemu_absent.clone(),
            ],
        ),
        component(
            "sptm",
            "SPTM",
            "Can the Secure Page Table Monitor run under the backend?",
            Status::Blocked,
            "The image is analysed, but the backend lacks most of the implementation-defined registers it uses",
            vec![
                decision(
                    format!("The SPTM component is {}", availability_label(sptm)),
                    SRC_ANALYSIS,
                    "static IM4P container",
                    Confidence::Target,
                    "Not executed",
                ),
                decision(
                    "In the A13-based model used by the fork, 98 of 127 SPTM implementation-defined registers are undefined",
                    SRC_SPTM,
                    "static register census",
                    Confidence::Target,
                    "Census of the static image. Lifetime classes are low confidence (indirect calls unresolved)",
                ),
                decision(
                    "The SPTM image is named for T8122 while the device tree is T8130",
                    SRC_ANALYSIS,
                    "static file name",
                    Confidence::Target,
                    "The naming discrepancy is recorded, not resolved",
                ),
            ],
        ),
        component(
            "txm",
            "TXM",
            "Can the Trusted Execution Monitor run under the backend?",
            Status::Blocked,
            "The TXM runs under the SPTM, which is itself blocked",
            vec![
                decision(
                    format!("The TXM component is {}", availability_label(txm)),
                    SRC_ANALYSIS,
                    "static IM4P container",
                    Confidence::Target,
                    "Not executed",
                ),
                decision(
                    "The TXM runs under the SPTM in the handoff order",
                    SRC_BOOT,
                    "static boot-chain analysis",
                    Confidence::Inferred,
                    "The order is taken from the boot-chain notes. Not observed at run time",
                ),
            ],
        ),
        component(
            "devicetree",
            "DeviceTree compatibility",
            "Can the target device tree be loaded by the backend?",
            Status::Partial,
            "The tree parses and its identity is correct. The backend cannot load it, and boot-time values are missing",
            vec![
                decision(
                    format!(
                        "The root compatible is {}",
                        dt.root_compatible.join(", ")
                    ),
                    SRC_ANALYSIS,
                    "static device tree",
                    Confidence::Target,
                    "Static template from the restore archive",
                ),
                decision(
                    format!("The tree has {} nodes", dt.node_count),
                    SRC_ANALYSIS,
                    "static device tree",
                    Confidence::Target,
                    "Node count of the binary payload",
                ),
            ],
        ),
        component(
            "kernel",
            "Kernel compatibility",
            "Can the target kernel boot on the target platform model?",
            Status::Blocked,
            "No boot has been observed. The kernel configuration token names T8122 while the platform is T8130, and no T8130 platform model exists",
            vec![
                decision(
                    format!(
                        "The kernel is a {} {} image with {} fileset entries",
                        kc.macho
                            .as_ref()
                            .map_or("unparsed".to_string(), |m| m.architecture.clone()),
                        kc.macho
                            .as_ref()
                            .map_or(String::new(), |m| m.filetype.clone()),
                        kc.fileset_entry_count
                    ),
                    SRC_ANALYSIS,
                    "static Mach-O",
                    Confidence::Target,
                    "Header and load commands only",
                ),
                decision(
                    format!(
                        "The kernel build reports {} with configuration {}",
                        kc.xnu_version.clone().unwrap_or_else(|| "no version".to_string()),
                        kc.configuration_token
                            .clone()
                            .unwrap_or_else(|| "no token".to_string())
                    ),
                    SRC_ANALYSIS,
                    "static kernel strings",
                    Confidence::Target,
                    "The T8122 token against the T8130 platform is unresolved",
                ),
            ],
        ),
        component(
            "rootfs",
            "Root filesystem availability",
            "Is a root filesystem available for the target?",
            Status::Partial,
            "The restore ramdisk is present and unencrypted. The system volume is not available locally",
            vec![
                decision(
                    format!("The restore ramdisk is {}", availability_label(ramdisk)),
                    SRC_ANALYSIS,
                    "static IM4P container",
                    Confidence::Target,
                    "A restore environment, not the installed system",
                ),
                decision(
                    format!("The system volume (OS) component is {}", availability_label(system)),
                    SRC_ANALYSIS,
                    "archive listing",
                    Confidence::Target,
                    "Its key service is external. No decryption was attempted",
                ),
            ],
        ),
        component(
            "gui",
            "GUI feasibility",
            "Can the target display an interface in the backend?",
            Status::Blocked,
            "No boot has been observed. The display, the GPU and the kernel are each blocked",
            vec![decision(
                "No target boot or display output has been observed",
                SRC_PROBE,
                "recorded probe",
                Confidence::Target,
                "The probes used a generic machine and are not target boots",
            )],
        ),
    ];

    Ok(FeasibilityMatrix {
        schema_version: SCHEMA_VERSION,
        target: "iPhone16,2 / D84AP / T8130 / iOS 27.0.1 (24A446)".to_string(),
        generated_on: date.to_string(),
        analysis_analysed_on: bundle.provenance.analysed_on.clone(),
        exact_target_vm: ExactTargetVm {
            status: "NOT_ESTABLISHED".to_string(),
            statement: "No exact target VM is established. The target has not booted in any backend, and the blockers are listed in the components above".to_string(),
        },
        components,
    })
}

/// Markdown rendering. Every status line carries its basis and its decisions.
pub fn markdown(m: &FeasibilityMatrix) -> String {
    use std::fmt::Write;
    let mut s = String::new();
    let _ = writeln!(s, "# T8130 feasibility report\n");
    let _ = writeln!(
        s,
        "Target: {}. Generated {} from the analysis of {}.\n",
        m.target, m.generated_on, m.analysis_analysed_on
    );
    let _ = writeln!(
        s,
        "**Exact target VM: {}.** {}\n",
        m.exact_target_vm.status, m.exact_target_vm.statement
    );
    let _ = writeln!(
        s,
        "Statuses: VERIFIED, PARTIAL, BLOCKED, UNSUPPORTED, UNVERIFIED. Confidence tags: [T] target artifact, [M] manifest, [I] inferred, [U] unknown. Nothing here is inferred from another SoC.\n"
    );
    s.push_str("| Component | Status | Question |\n|---|---|---|\n");
    for c in &m.components {
        let _ = writeln!(s, "| {} | {} | {} |", c.title, c.status.label(), c.question);
    }
    s.push('\n');
    for c in &m.components {
        let _ = writeln!(s, "## {}\n", c.title);
        let _ = writeln!(s, "**{}.** {}\n", c.status.label(), c.basis);
        for d in &c.decisions {
            let _ = writeln!(
                s,
                "- [{}] {} (source: {}; evidence: {}; verified {}). Limitation: {}",
                d.confidence.tag(),
                d.statement,
                d.source,
                d.evidence_type,
                d.last_verified,
                d.limitation
            );
        }
        s.push('\n');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_labels_match_the_vocabulary() {
        let labels: Vec<&str> = [
            Status::Verified,
            Status::Partial,
            Status::Blocked,
            Status::Unsupported,
            Status::Unverified,
        ]
        .iter()
        .map(|s| s.label())
        .collect();
        assert_eq!(
            labels,
            vec![
                "VERIFIED",
                "PARTIAL",
                "BLOCKED",
                "UNSUPPORTED",
                "UNVERIFIED"
            ]
        );
    }
}
