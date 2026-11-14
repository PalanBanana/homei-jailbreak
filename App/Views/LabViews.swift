import SwiftUI
import HomeiKit

// Virtual research lab screens. They read the sanitized `lab-state.json` exported by the host
// controller. The app never starts a virtual machine, never connects to a device and never runs
// Ghidra or a package manager. The controls below are explanations, not buttons.

enum LabText {
    /// Localised text for a key that is built at run time. A test checks that every key exists.
    static func loc(_ key: String) -> String { Bundle.main.localizedString(forKey: key, value: key, table: nil) }
    static func yesNo(_ value: Bool) -> String { loc(value ? "lab.yes" : "lab.no") }
    static func shortHash(_ hash: String) -> String { hash.count > 16 ? String(hash.prefix(16)) + "…" : hash }

    static func size(_ bytes: UInt64) -> String {
        ByteCountFormatter.string(fromByteCount: Int64(clamping: bytes), countStyle: .file)
    }
}

// MARK: Components

/// A status with colour, symbol and text. Colour is never the only signal, and only the exact-target
/// status is ever drawn as positive.
struct LabStatusBadge: View {
    let textKey: String
    let color: Color
    let symbol: String

    var body: some View {
        Label(LocalizedStringKey(textKey), systemImage: symbol)
            .font(.subheadline.weight(.semibold))
            .padding(.horizontal, 12).padding(.vertical, 6)
            .foregroundStyle(color)
            .background(color.opacity(0.16), in: Capsule())
            .fixedSize(horizontal: false, vertical: true)
            .accessibilityLabel(Text(LocalizedStringKey(textKey)))
    }

    static func vm(_ status: LabVmStatus) -> LabStatusBadge {
        switch status {
        case .exactTargetVerified:
            return LabStatusBadge(textKey: status.localizationKey, color: Theme.positive, symbol: "checkmark.seal")
        case .targetKernelBootedHardwareUnverified, .partialDarwinGuest:
            return LabStatusBadge(textKey: status.localizationKey, color: Theme.caution, symbol: "circle.lefthalf.filled")
        case .blockedByMissingFirmware, .blockedByMissingQemuFeature, .bootFailure, .unsupportedTarget:
            return LabStatusBadge(textKey: status.localizationKey, color: Theme.negative, symbol: "xmark.octagon")
        case .notTested, .referenceDeviceOnly:
            return LabStatusBadge(textKey: status.localizationKey, color: Theme.neutral, symbol: "circle.dotted")
        }
    }

    static func fidelity(_ status: LabFidelityStatus) -> LabStatusBadge {
        switch status {
        case .exact: return LabStatusBadge(textKey: status.localizationKey, color: Theme.positive, symbol: "checkmark.seal")
        case .partial, .approximate: return LabStatusBadge(textKey: status.localizationKey, color: Theme.caution, symbol: "circle.lefthalf.filled")
        case .unsupported: return LabStatusBadge(textKey: status.localizationKey, color: Theme.negative, symbol: "xmark.octagon")
        case .unverified, .notTested: return LabStatusBadge(textKey: status.localizationKey, color: Theme.neutral, symbol: "questionmark.diamond")
        }
    }

    static func experiment(_ status: LabExperimentStatus) -> LabStatusBadge {
        switch status {
        case .completed: return LabStatusBadge(textKey: status.localizationKey, color: Theme.accent, symbol: "checkmark.circle")
        case .failed: return LabStatusBadge(textKey: status.localizationKey, color: Theme.negative, symbol: "xmark.octagon")
        case .interrupted, .inconclusive, .notReproducible:
            return LabStatusBadge(textKey: status.localizationKey, color: Theme.caution, symbol: "exclamationmark.triangle")
        case .planned, .running: return LabStatusBadge(textKey: status.localizationKey, color: Theme.neutral, symbol: "circle.dotted")
        }
    }
}

/// A line of prose from the data files, drawn as secondary text.
struct LabNote: View {
    let text: String
    var body: some View {
        Text(text).font(.footnote).foregroundStyle(Theme.textSecondary).fixedSize(horizontal: false, vertical: true)
    }
}

/// Shown when the lab state is missing or invalid. Nothing is guessed in its place.
struct LabUnavailableView: View {
    @EnvironmentObject var model: AppModel
    var body: some View {
        DataErrorView(message: model.labError ?? "")
    }
}

/// Wraps a lab screen so each one handles a missing state the same way.
struct LabScreen<Content: View>: View {
    @EnvironmentObject var model: AppModel
    let titleKey: String
    let content: (LabState) -> Content

    init(titleKey: String, @ViewBuilder content: @escaping (LabState) -> Content) {
        self.titleKey = titleKey
        self.content = content
    }

    var body: some View {
        Screen(titleKey: titleKey) {
            if let lab = model.lab {
                content(lab)
            } else {
                LabUnavailableView()
            }
        }
    }
}

// MARK: 1. Overview

struct LabHomeView: View {
    var body: some View {
        LabScreen(titleKey: "lab.title") { lab in
            DataKindBanner(textKey: "lab.banner", symbol: "cube.transparent")
            LabStatusBadge.vm(lab.fidelity.vmStatus)
            Text(LocalizedStringKey(lab.fidelity.vmStatus.detailLocalizationKey))
                .font(.footnote).foregroundStyle(Theme.textSecondary).fixedSize(horizontal: false, vertical: true)
            SectionCard(titleKey: "lab.overview") {
                InfoRow(labelKey: "lab.target", value: "\(lab.target.productType) · \(lab.target.osVersion) (\(lab.target.buildVersion))")
                InfoRow(labelKey: "lab.exact_vm", value: LabText.yesNo(lab.claimsExactTargetVM))
                InfoRow(labelKey: "lab.vm_running", value: LabText.yesNo(lab.guest.vmRunning))
                InfoRow(labelKey: "lab.generated_on", value: lab.generatedOn)
            }
            SectionCard(titleKey: "lab.screens") {
                LabLinkRow(titleKey: "lab.nav.target") { LabTargetView() }
                LabLinkRow(titleKey: "lab.nav.backend") { LabBackendView() }
                LabLinkRow(titleKey: "lab.nav.firmware") { LabFirmwareView() }
                LabLinkRow(titleKey: "lab.nav.ghidra") { LabGhidraView() }
                LabLinkRow(titleKey: "lab.nav.guest") { LabGuestView() }
                LabLinkRow(titleKey: "lab.nav.packages") { LabPackagesView() }
                LabLinkRow(titleKey: "lab.nav.bootstrap") { LabBootstrapView() }
                LabLinkRow(titleKey: "lab.nav.experiments") { LabExperimentsView() }
                LabLinkRow(titleKey: "lab.nav.fidelity") { LabFidelityView() }
            }
        }
    }
}

/// A navigation row with a 44 pt hit target. Text and chevron keep their own explicit colours.
struct LabLinkRow<Destination: View>: View {
    let titleKey: String
    let destination: () -> Destination

    init(titleKey: String, @ViewBuilder destination: @escaping () -> Destination) {
        self.titleKey = titleKey
        self.destination = destination
    }

    var body: some View {
        NavigationLink {
            destination()
        } label: {
            HStack {
                Text(LocalizedStringKey(titleKey)).foregroundStyle(Theme.textPrimary)
                Spacer(minLength: 8)
                Image(systemName: "chevron.right").foregroundStyle(Theme.textSecondary).accessibilityHidden(true)
            }
            .frame(minHeight: 44)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }
}

// MARK: 2. Target profile

struct LabTargetView: View {
    var body: some View {
        LabScreen(titleKey: "lab.nav.target") { lab in
            let t = lab.target
            DataKindBanner(textKey: "lab.target.banner", symbol: "iphone")
            SectionCard(titleKey: "lab.target.identity") {
                InfoRow(labelKey: "lab.target.model", value: t.commercialModel)
                InfoRow(labelKey: "lab.target.product_type", value: t.productType)
                InfoRow(labelKey: "lab.target.soc", value: t.soc)
                InfoRow(labelKey: "lab.target.cpu", value: t.cpu)
                InfoRow(labelKey: "lab.target.board", value: t.board)
                InfoRow(labelKey: "lab.target.cpid", value: t.cpid)
                InfoRow(labelKey: "lab.target.bdid", value: t.bdid)
                InfoRow(labelKey: "lab.target.architecture", value: t.architecture)
                InfoRow(labelKey: "lab.target.os", value: "\(t.osVersion) (\(t.buildVersion))")
                InfoRow(labelKey: "lab.target.checked_on", value: t.checkedOn)
            }
            SectionCard(titleKey: "lab.target.reference_status") {
                InfoRow(labelKey: "lab.target.vm_status", value: LabText.loc(t.vmStatus.localizationKey))
                InfoRow(labelKey: "lab.target.hardware_fidelity", value: LabText.loc(t.hardwareFidelity.localizationKey))
                LabNote(text: LabText.loc("lab.target.reference_note"))
            }
            SectionCard(titleKey: "lab.target.provenance") {
                ForEach(LabTarget.provenanceFields, id: \.self) { field in
                    if let p = t.provenance[field] {
                        VStack(alignment: .leading, spacing: 2) {
                            Text(verbatim: field).font(.subheadline.weight(.semibold)).foregroundStyle(Theme.textPrimary)
                            LabNote(text: p.source)
                            Text(verbatim: "\(p.kind.rawValue) · \(p.confidence.rawValue)")
                                .font(.caption).foregroundStyle(Theme.textSecondary)
                        }
                        .accessibilityElement(children: .combine)
                    }
                }
            }
            if !t.discrepancies.isEmpty {
                SectionCard(titleKey: "lab.target.discrepancies") {
                    ForEach(t.discrepancies) { d in
                        VStack(alignment: .leading, spacing: 2) {
                            Text(verbatim: d.id).font(.subheadline.weight(.semibold)).foregroundStyle(Theme.textPrimary)
                            LabNote(text: d.summary)
                        }
                        .accessibilityElement(children: .combine)
                    }
                }
            }
        }
    }
}

// MARK: 3. VM backend

struct LabBackendView: View {
    private let dryRunCommand = "homei-lab plan --firmware <firmware-dir>"
    private let preflightCommand = "homei-lab preflight"

    var body: some View {
        LabScreen(titleKey: "lab.nav.backend") { lab in
            DataKindBanner(textKey: "lab.backend.banner", symbol: "server.rack")
            LabStatusBadge.vm(lab.fidelity.vmStatus)
            ForEach(lab.backends) { b in
                SectionCard(verbatimTitle: b.name) {
                    InfoRow(labelKey: "lab.backend.available", value: LabText.yesNo(b.available))
                    InfoRow(labelKey: "lab.backend.optional", value: LabText.yesNo(b.optional))
                    InfoRow(labelKey: "lab.backend.status", value: LabText.loc(b.vmStatus.localizationKey))
                    if let commit = b.upstreamCommit { LabNote(text: commit) }
                    if !b.blockers.isEmpty {
                        Text("lab.backend.blockers").font(.subheadline.weight(.semibold)).foregroundStyle(Theme.textPrimary)
                        ForEach(b.blockers, id: \.self) { LabNote(text: "• \($0)") }
                    }
                }
            }
            SectionCard(titleKey: "lab.backend.host") {
                InfoRow(labelKey: "lab.backend.host_system", value: "\(lab.host.distribution) \(lab.host.architecture)")
                InfoRow(labelKey: "lab.backend.qemu", value: lab.host.qemuVersion.map { "QEMU \($0)" } ?? LabText.loc("lab.unknown"))
                InfoRow(labelKey: "lab.backend.accelerators", value: lab.host.qemuAccelerators.isEmpty ? LabText.loc("lab.none") : lab.host.qemuAccelerators.joined(separator: ", "))
                InfoRow(labelKey: "lab.backend.kvm", value: LabText.yesNo(lab.host.kvmDevicePresent))
                InfoRow(labelKey: "lab.backend.memory", value: "\(lab.host.memoryAvailableGib) / \(lab.host.memoryTotalGib) GiB")
                LabNote(text: LabText.loc("lab.backend.kvm_note"))
            }
            // These are explanations. There is no control in the app that starts or stops a guest.
            SectionCard(titleKey: "lab.backend.controls") {
                LabControlRow(titleKey: "lab.control.start", symbol: "play.circle")
                LabControlRow(titleKey: "lab.control.stop", symbol: "stop.circle")
                LabControlRow(titleKey: "lab.control.snapshot", symbol: "camera.circle")
                LabNote(text: LabText.loc("lab.control.why_unavailable"))
                Text("lab.control.dry_run").font(.subheadline.weight(.semibold)).foregroundStyle(Theme.textPrimary)
                Text(verbatim: preflightCommand).font(.system(.footnote, design: .monospaced)).foregroundStyle(Theme.textPrimary)
                Text(verbatim: dryRunCommand).font(.system(.footnote, design: .monospaced)).foregroundStyle(Theme.textPrimary)
                LabNote(text: LabText.loc("lab.control.explicit"))
            }
        }
    }
}

/// A control that is shown but unavailable. It is deliberately not a button.
struct LabControlRow: View {
    let titleKey: String
    let symbol: String

    var body: some View {
        HStack {
            Label(LocalizedStringKey(titleKey), systemImage: symbol).foregroundStyle(Theme.textPrimary)
            Spacer(minLength: 8)
            Label("lab.control.unavailable", systemImage: "lock").font(.footnote).foregroundStyle(Theme.caution)
        }
        .font(.subheadline)
        .accessibilityElement(children: .combine)
    }
}

// MARK: 4. Firmware artifacts

struct LabFirmwareView: View {
    var body: some View {
        LabScreen(titleKey: "lab.nav.firmware") { lab in
            DataKindBanner(textKey: "lab.firmware.banner", symbol: "doc.badge.gearshape")
            LabNote(text: LabText.loc("lab.firmware.checksum_note"))
            ForEach(lab.firmware) { f in
                SectionCard(verbatimTitle: f.name) {
                    InfoRow(labelKey: "lab.firmware.role", value: f.role)
                    InfoRow(labelKey: "lab.firmware.format", value: f.format)
                    InfoRow(labelKey: "lab.firmware.size", value: LabText.size(f.bytes))
                    InfoRow(labelKey: "lab.firmware.build", value: f.sourceBuild)
                    InfoRow(labelKey: "lab.firmware.sha256", value: LabText.shortHash(f.sha256))
                }
            }
        }
    }
}

// MARK: 5. Ghidra analysis

struct LabGhidraView: View {
    var body: some View {
        LabScreen(titleKey: "lab.nav.ghidra") { lab in
            let g = lab.ghidra
            DataKindBanner(textKey: "lab.ghidra.banner", symbol: "doc.text.magnifyingglass")
            SectionCard(titleKey: "lab.ghidra.project") {
                InfoRow(labelKey: "lab.ghidra.tool", value: g.ghidraVersion ?? LabText.loc("lab.unknown"))
                InfoRow(labelKey: "lab.ghidra.project_present", value: LabText.yesNo(g.projectPresent))
                InfoRow(labelKey: "lab.ghidra.program", value: g.program)
                InfoRow(labelKey: "lab.ghidra.architecture", value: g.architecture)
                InfoRow(labelKey: "lab.ghidra.firmware_sha256", value: g.firmwareSha256.map(LabText.shortHash) ?? LabText.loc("lab.unknown"))
            }
            SectionCard(titleKey: "lab.ghidra.analysis") {
                InfoRow(labelKey: "lab.ghidra.functions", value: "\(g.functionCount)")
                InfoRow(labelKey: "lab.ghidra.fileset", value: "\(g.filesetEntriesCovered) / \(g.filesetEntries)")
                InfoRow(labelKey: "lab.ghidra.symbol_match", value: g.symbolMatchText ?? LabText.loc("lab.unknown"))
                InfoRow(labelKey: "lab.ghidra.last_analysis", value: g.lastAnalysisDate ?? LabText.loc("lab.unknown"))
                InfoRow(labelKey: "lab.ghidra.confidence", value: g.confidence)
                LabNote(text: g.importStatus)
            }
            SectionCard(titleKey: "lab.ghidra.issues") {
                if g.importErrors.isEmpty {
                    Text("lab.none").foregroundStyle(Theme.textSecondary)
                } else {
                    Text(verbatim: "\(g.importErrors.count)").font(.subheadline.weight(.semibold)).foregroundStyle(Theme.textPrimary)
                    ForEach(Array(g.importErrors.prefix(6)), id: \.self) { LabNote(text: "• \($0)") }
                }
            }
            LabNote(text: LabText.loc("lab.ghidra.read_only"))
            LabNote(text: LabText.loc("lab.ghidra.no_claims"))
        }
    }
}

// MARK: 6. Guest workspace and snapshots

struct LabGuestView: View {
    var body: some View {
        LabScreen(titleKey: "lab.nav.guest") { lab in
            let g = lab.guest
            DataKindBanner(textKey: "lab.guest.banner", symbol: "externaldrive")
            SectionCard(titleKey: "lab.guest.workspace") {
                InfoRow(labelKey: "lab.guest.scope", value: g.filesystemScope)
                InfoRow(labelKey: "lab.guest.overlay", value: LabText.yesNo(g.overlayPresent))
                InfoRow(labelKey: "lab.guest.base_image", value: g.baseImageSha256.map(LabText.shortHash) ?? LabText.loc("lab.none"))
                InfoRow(labelKey: "lab.guest.snapshots", value: "\(g.snapshots)")
                InfoRow(labelKey: "lab.guest.running", value: LabText.yesNo(g.vmRunning))
            }
            SectionCard(titleKey: "lab.guest.roots") {
                InfoRow(labelKey: "lab.guest.guest_root", value: LabPrivilegeScope.guestRoot.rawValue)
                InfoRow(labelKey: "lab.guest.device_root", value: LabPrivilegeScope.physicalDeviceRoot.rawValue)
                LabNote(text: LabText.loc("lab.guest.roots_note"))
            }
            SectionCard(titleKey: "lab.guest.isolation") {
                LabNote(text: LabText.loc("lab.guest.isolation_note"))
                LabNote(text: LabText.loc("lab.guest.delete_note"))
            }
        }
    }
}

// MARK: 7. Virtual packages

struct LabPackagesView: View {
    var body: some View {
        LabScreen(titleKey: "lab.nav.packages") { lab in
            DataKindBanner(textKey: "lab.packages.banner", symbol: "shippingbox")
            LabNote(text: LabText.loc("lab.packages.rules"))
            InfoRow(labelKey: "lab.packages.transactions", value: "\(lab.guest.transactions)")
            if lab.guest.packages.isEmpty {
                Text("lab.packages.none").foregroundStyle(Theme.textSecondary)
            }
            ForEach(lab.guest.packages) { p in
                SectionCard(verbatimTitle: "\(p.id) \(p.version)") {
                    InfoRow(labelKey: "lab.packages.signature", value: p.signature)
                    InfoRow(labelKey: "lab.packages.files", value: "\(p.files)")
                    LabNote(text: p.provenance)
                }
            }
            LabNote(text: LabText.loc("lab.packages.test_key"))
        }
    }
}

// MARK: 8. Guest bootstrap

struct LabBootstrapView: View {
    var body: some View {
        LabScreen(titleKey: "lab.nav.bootstrap") { lab in
            DataKindBanner(textKey: "lab.bootstrap.banner", symbol: "testtube.2")
            SectionCard(titleKey: "lab.bootstrap.state") {
                InfoRow(labelKey: "lab.bootstrap.version", value: lab.guest.bootstrapVersion ?? LabText.loc("lab.none"))
                InfoRow(labelKey: "lab.bootstrap.status", value: lab.guest.bootstrapState)
                InfoRow(labelKey: "lab.bootstrap.packages", value: "\(lab.guest.packages.count)")
            }
            SectionCard(titleKey: "lab.bootstrap.scope") {
                LabNote(text: LabText.loc("lab.bootstrap.scope_note"))
                LabNote(text: LabText.loc("lab.bootstrap.not_ios"))
            }
        }
    }
}

// MARK: 9. Experiments

struct LabExperimentsView: View {
    var body: some View {
        LabScreen(titleKey: "lab.nav.experiments") { lab in
            DataKindBanner(textKey: "lab.experiments.banner", symbol: "list.bullet.clipboard")
            if lab.experiments.isEmpty {
                Text("lab.experiments.none").foregroundStyle(Theme.textSecondary)
            }
            ForEach(lab.experiments) { e in
                SectionCard(verbatimTitle: e.experimentId) {
                    LabStatusBadge.experiment(e.status)
                    InfoRow(labelKey: "lab.experiments.kind", value: e.kind)
                    InfoRow(labelKey: "lab.experiments.backend", value: "\(e.backend.name) · \(LabText.loc(e.backend.kind.localizationKey))")
                    InfoRow(labelKey: "lab.experiments.target", value: "\(e.target.productType) · \(e.target.buildVersion)")
                    InfoRow(labelKey: "lab.experiments.guest_build", value: e.guestOsBuild ?? LabText.loc("lab.not_established"))
                    InfoRow(labelKey: "lab.experiments.boot", value: e.bootOutcome.map { LabText.loc($0.localizationKey) } ?? LabText.loc("lab.none"))
                    InfoRow(labelKey: "lab.experiments.time", value: e.timestamp)
                    InfoRow(labelKey: "lab.experiments.config", value: LabText.shortHash(e.configSha256))
                    if e.backend.kind == .hostOnly {
                        Label("lab.experiments.no_vm", systemImage: "info.circle").font(.footnote).foregroundStyle(Theme.accent)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                    LabNote(text: e.result)
                    ForEach(e.limitations, id: \.self) { LabNote(text: "• \($0)") }
                }
            }
        }
    }
}

// MARK: 10. Fidelity

struct LabFidelityView: View {
    var body: some View {
        LabScreen(titleKey: "lab.nav.fidelity") { lab in
            let f = lab.fidelity
            DataKindBanner(textKey: "lab.fidelity.banner", symbol: "chart.bar.doc.horizontal")
            LabStatusBadge.vm(f.vmStatus)
            SectionCard(titleKey: "lab.fidelity.firmware_axis") {
                LabStatusBadge.fidelity(f.firmware)
                InfoRow(labelKey: "lab.fidelity.identity", value: LabText.loc(f.firmwareIdentity.localizationKey))
                LabNote(text: LabText.loc("lab.fidelity.firmware_note"))
            }
            SectionCard(titleKey: "lab.fidelity.hardware_axis") {
                LabStatusBadge.fidelity(f.hardware)
                InfoRow(labelKey: "lab.fidelity.hardware_level", value: LabText.loc(f.hardwareFidelity.localizationKey))
                LabNote(text: LabText.loc("lab.fidelity.hardware_note"))
            }
            SectionCard(titleKey: "lab.fidelity.functional_axis") {
                ForEach(LabState.featureOrder, id: \.self) { name in
                    if let status = f.functional[name] {
                        InfoRow(labelKey: "lab.feature.\(name)", value: LabText.loc(status.localizationKey))
                    }
                }
            }
            SectionCard(titleKey: "lab.fidelity.notes") {
                ForEach(f.notes, id: \.self) { LabNote(text: "• \($0)") }
            }
            if !lab.unresolved.isEmpty {
                SectionCard(titleKey: "lab.fidelity.unresolved") {
                    ForEach(lab.unresolved, id: \.self) { LabNote(text: "• \($0)") }
                }
            }
            if !lab.evidence.isEmpty {
                SectionCard(titleKey: "lab.fidelity.evidence") {
                    ForEach(lab.evidence) { e in
                        VStack(alignment: .leading, spacing: 2) {
                            Text(verbatim: "\(e.id) · \(e.title)").font(.subheadline.weight(.semibold)).foregroundStyle(Theme.textPrimary)
                            LabNote(text: e.reference)
                            LabNote(text: e.limitation)
                        }
                        .accessibilityElement(children: .combine)
                    }
                }
            }
        }
    }
}
