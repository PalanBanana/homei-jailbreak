import SwiftUI
import HomeiKit

// Firmware research screens. They read the bundled analysis, feasibility matrix and register census
// that the host tools exported and checked. Each value is shown as recorded, with its confidence tag
// and its source. A missing or invalid file is an error state. The screens never draw a success
// that the data does not establish, and they never claim an exact target virtual machine.

enum ResearchText {
    /// A short, fixed-format hash prefix for display.
    static func short(_ hash: String) -> String { hash.count > 16 ? String(hash.prefix(16)) + "…" : hash }

    static func bytes(_ value: Int64) -> String {
        ByteCountFormatter.string(fromByteCount: value, countStyle: .file)
    }

    static func yesNo(_ value: Bool) -> String {
        Bundle.main.localizedString(forKey: value ? "lab.yes" : "lab.no", value: "", table: nil)
    }

    static func color(_ status: ResearchStatus) -> Color {
        switch status {
        case .verified: return Theme.positive
        case .partial: return Theme.caution
        case .blocked, .unsupported: return Theme.negative
        case .unverified: return Theme.neutral
        }
    }

    static func symbol(_ status: ResearchStatus) -> String {
        switch status {
        case .verified: return "checkmark.seal"
        case .partial: return "circle.lefthalf.filled"
        case .blocked: return "xmark.octagon"
        case .unsupported: return "nosign"
        case .unverified: return "questionmark.diamond"
        }
    }

    static func color(_ availability: ResearchAvailability) -> Color {
        switch availability {
        case .analysed: return Theme.positive
        case .containerOnly, .unsupported: return Theme.caution
        case .encrypted, .missing: return Theme.negative
        case .unavailable, .noManifestPath: return Theme.neutral
        }
    }

    static func symbol(_ availability: ResearchAvailability) -> String {
        switch availability {
        case .analysed: return "checkmark.circle"
        case .containerOnly: return "shippingbox"
        case .encrypted: return "lock"
        case .unsupported: return "questionmark.circle"
        case .unavailable: return "tray"
        case .missing: return "exclamationmark.triangle"
        case .noManifestPath: return "doc.questionmark"
        }
    }
}

/// A status with colour, symbol and text. Colour is never the only signal.
struct ResearchBadge: View {
    let textKey: String
    let color: Color
    let symbol: String

    var body: some View {
        Label(LocalizedStringKey(textKey), systemImage: symbol)
            .font(.subheadline.weight(.semibold))
            .foregroundStyle(color)
            .fixedSize(horizontal: false, vertical: true)
            .accessibilityElement(children: .combine)
    }
}

/// Loads the bundled research files once, off the main thread, and shows a loading state meanwhile.
struct ResearchScreen<Content: View>: View {
    let titleKey: String
    @ViewBuilder let content: (ResearchCatalog) -> Content
    @State private var result: Result<ResearchCatalog, ResearchLoadError>?

    init(titleKey: String, @ViewBuilder content: @escaping (ResearchCatalog) -> Content) {
        self.titleKey = titleKey
        self.content = content
    }

    var body: some View {
        Screen(titleKey: titleKey) {
            switch result {
            case .none:
                ProgressView("research.loading")
                    .frame(maxWidth: .infinity, minHeight: 120)
                    .accessibilityLabel(Text(LocalizedStringKey("research.loading")))
            case .some(.success(let catalog)):
                content(catalog)
            case .some(.failure):
                SectionCard(titleKey: "research.load_error.title") {
                    ResearchBadge(textKey: "research.load_error.title", color: Theme.negative, symbol: "xmark.octagon")
                    Text(LocalizedStringKey("research.load_error.detail"))
                        .font(.footnote).foregroundStyle(Theme.textSecondary)
                }
            }
        }
        .task {
            if result == nil {
                result = await Task.detached(priority: .userInitiated) {
                    ResearchCatalog.loadFromBundle()
                }.value
            }
        }
    }
}

// MARK: Hub

struct ResearchHomeView: View {
    var body: some View {
        ResearchScreen(titleKey: "research.title") { catalog in
            DataKindBanner(textKey: "research.banner", symbol: "doc.text.magnifyingglass")
            SectionCard(titleKey: "research.target") {
                InfoRow(labelKey: "research.target.product", value: catalog.analysis.target.productType)
                InfoRow(labelKey: "research.target.board", value: catalog.analysis.target.boardConfig)
                InfoRow(labelKey: "research.target.chip", value: catalog.analysis.target.chip)
                InfoRow(labelKey: "research.target.build", value: "\(catalog.analysis.target.productVersion) (\(catalog.analysis.target.build))")
                InfoRow(labelKey: "research.exact_vm", value: catalog.matrix.exactTargetVm.status)
            }
            SectionCard(titleKey: "research.status_counts") {
                ForEach(catalog.statusCounts, id: \.0) { status, count in
                    HStack {
                        ResearchBadge(textKey: status.localizationKey, color: ResearchText.color(status), symbol: ResearchText.symbol(status))
                        Spacer(minLength: 8)
                        Text("\(count)").foregroundStyle(Theme.textPrimary).monospacedDigit()
                    }
                    .frame(minHeight: 44)
                }
            }
            SectionCard(titleKey: "research.screens") {
                LabLinkRow(titleKey: "research.nav.firmware") { ResearchFirmwareView() }
                LabLinkRow(titleKey: "research.nav.devicetree") { ResearchDeviceTreeView() }
                LabLinkRow(titleKey: "research.nav.kernelcache") { ResearchKernelcacheView() }
                LabLinkRow(titleKey: "research.nav.ghidra") { ResearchGhidraView() }
                LabLinkRow(titleKey: "research.nav.sptm") { ResearchSPTMView() }
                LabLinkRow(titleKey: "research.nav.aicpmgr") { ResearchAICPMGRView() }
                LabLinkRow(titleKey: "research.nav.vm") { ResearchVMView() }
                LabLinkRow(titleKey: "research.nav.experiments") { ResearchExperimentsView() }
                LabLinkRow(titleKey: "research.nav.compatibility") { ResearchCompatibilityView() }
                LabLinkRow(titleKey: "research.nav.evidence") { ResearchEvidenceView() }
            }
        }
    }
}

// MARK: 1. Firmware explorer

struct ResearchFirmwareView: View {
    var body: some View {
        ResearchScreen(titleKey: "research.firmware.title") { catalog in
            let s = catalog.analysis.summary
            SectionCard(titleKey: "research.firmware.summary") {
                InfoRow(labelKey: "research.firmware.components", value: "\(s.eraseComponents)")
                InfoRow(labelKey: "research.availability.analysed", value: "\(s.analysed)")
                InfoRow(labelKey: "research.availability.encrypted", value: "\(s.encrypted)")
                InfoRow(labelKey: "research.availability.unavailable", value: "\(s.unavailable)")
                InfoRow(labelKey: "research.availability.no_manifest_path", value: "\(s.noManifestPath)")
                InfoRow(labelKey: "research.availability.missing", value: "\(s.missing)")
                InfoRow(labelKey: "research.firmware.archive_entries", value: "\(catalog.analysis.archive.entryCount)")
            }
            SectionCard(titleKey: "research.firmware.identity") {
                ForEach(catalog.analysis.identityChecks, id: \.id) { check in
                    InfoRow(labelKey: "research.identity.\(check.id)", value: check.passed ? "✓ \(check.observed)" : "✗ \(check.observed)")
                }
            }
            SectionCard(titleKey: "research.firmware.components") {
                ForEach(catalog.analysis.components) { c in
                    VStack(alignment: .leading, spacing: 4) {
                        Text(verbatim: c.name).font(.subheadline.weight(.semibold)).foregroundStyle(Theme.textPrimary)
                        ResearchBadge(textKey: c.availability.localizationKey,
                                      color: ResearchText.color(c.availability),
                                      symbol: ResearchText.symbol(c.availability))
                        if let k = c.container?.fourcc {
                            InfoRow(labelKey: "research.firmware.fourcc", value: k)
                        }
                        if let sha = c.localSha256 {
                            InfoRow(labelKey: "research.firmware.sha256", value: ResearchText.short(sha))
                        }
                        if let d = c.manifestDigestMatches {
                            InfoRow(labelKey: "research.firmware.digest_matches", value: ResearchText.yesNo(d))
                        }
                    }
                    .padding(.vertical, 6)
                    .accessibilityElement(children: .combine)
                }
            }
            SectionCard(titleKey: "research.firmware.provenance") {
                ForEach(catalog.analysis.provenance.inputs, id: \.fileName) { input in
                    InfoRow(labelKey: "research.firmware.input", value: "\(input.role): \(input.fileName)")
                    InfoRow(labelKey: "research.firmware.sha256", value: ResearchText.short(input.sha256))
                }
                ForEach(catalog.analysis.provenance.externalSteps, id: \.self) { step in
                    LabNote(text: step)
                }
            }
        }
    }
}

// MARK: 2. Device tree inspector

struct ResearchDeviceTreeView: View {
    var body: some View {
        ResearchScreen(titleKey: "research.devicetree.title") { catalog in
            if let dt = catalog.analysis.devicetree {
                SectionCard(titleKey: "research.devicetree.root") {
                    InfoRow(labelKey: "research.devicetree.nodes", value: "\(dt.nodeCount)")
                    InfoRow(labelKey: "research.devicetree.compatible", value: dt.rootCompatible.joined(separator: ", "))
                    InfoRow(labelKey: "research.devicetree.model", value: dt.rootModel ?? "-")
                    InfoRow(labelKey: "research.devicetree.target_type", value: dt.rootTargetType ?? "-")
                    InfoRow(labelKey: "research.devicetree.arm_io", value: dt.armIoCompatible.joined(separator: ", "))
                }
                SectionCard(titleKey: "research.devicetree.ranges") {
                    ForEach(Array(dt.armIoRanges.enumerated()), id: \.offset) { _, r in
                        InfoRow(labelKey: "research.devicetree.range", value: "\(r.childBaseHex) → \(r.parentBaseHex) (\(r.sizeHex))")
                    }
                }
                SectionCard(titleKey: "research.devicetree.payload") {
                    InfoRow(labelKey: "research.firmware.sha256", value: ResearchText.short(dt.payloadSha256))
                    InfoRow(labelKey: "research.devicetree.bytes", value: ResearchText.bytes(dt.payloadBytes))
                }
            } else {
                ResearchMissingCard(titleKey: "research.devicetree.title")
            }
        }
    }
}

// MARK: 3. Kernelcache inspector

struct ResearchKernelcacheView: View {
    var body: some View {
        ResearchScreen(titleKey: "research.kernelcache.title") { catalog in
            if let k = catalog.analysis.kernelcache {
                SectionCard(titleKey: "research.kernelcache.header") {
                    if let m = k.macho {
                        InfoRow(labelKey: "research.kernelcache.architecture", value: m.architecture)
                        InfoRow(labelKey: "research.kernelcache.filetype", value: m.filetype)
                        InfoRow(labelKey: "research.kernelcache.cpusubtype", value: m.cpusubtypeHex)
                        InfoRow(labelKey: "research.kernelcache.load_commands", value: "\(m.loadCommandCount)")
                    } else {
                        ResearchBadge(textKey: "research.kernelcache.parse_failed", color: Theme.negative, symbol: "xmark.octagon")
                        LabNote(text: k.machoError ?? "")
                    }
                    InfoRow(labelKey: "research.kernelcache.xnu", value: k.xnuVersion ?? "-")
                    InfoRow(labelKey: "research.kernelcache.configuration", value: k.configurationToken ?? "-")
                    InfoRow(labelKey: "research.devicetree.bytes", value: ResearchText.bytes(k.payloadBytes))
                }
                if let m = k.macho {
                    SectionCard(titleKey: "research.kernelcache.segments") {
                        ForEach(m.segments, id: \.name) { seg in
                            InfoRow(labelKey: seg.name, value: "\(String(seg.vmaddr, radix: 16)) · \(seg.sections)")
                        }
                    }
                }
                SectionCard(titleKey: "research.kernelcache.filesets") {
                    InfoRow(labelKey: "research.kernelcache.fileset_count", value: "\(k.filesetEntryCount)")
                    ForEach(k.filesetEntries, id: \.self) { entry in
                        Text(verbatim: entry).font(.footnote).foregroundStyle(Theme.textSecondary)
                    }
                }
            } else {
                ResearchMissingCard(titleKey: "research.kernelcache.title")
            }
        }
    }
}

// MARK: 4. Ghidra analysis browser

struct ResearchGhidraView: View {
    var body: some View {
        ResearchScreen(titleKey: "research.ghidra.title") { catalog in
            if let g = catalog.analysis.ghidra {
                SectionCard(titleKey: "research.ghidra.project") {
                    InfoRow(labelKey: "research.ghidra.name", value: g.projectName)
                    InfoRow(labelKey: "research.ghidra.file_state", value: g.projectFileState)
                    InfoRow(labelKey: "research.ghidra.repository", value: ResearchText.yesNo(g.repositoryPresent))
                    InfoRow(labelKey: "research.ghidra.index_files", value: "\(g.indexFiles)")
                    InfoRow(labelKey: "research.ghidra.function_count", value: g.functionCount.map { "\($0)" } ?? String(localized: "research.ghidra.not_exported"))
                }
                SectionCard(titleKey: "research.ghidra.names") {
                    LabNote(text: String(localized: "research.ghidra.names_note"))
                    ForEach(g.names, id: \.name) { n in
                        InfoRow(labelKey: "research.ghidra.name_row", value: "\(n.name): \(ResearchText.yesNo(n.presentInIndex))")
                    }
                }
            } else {
                ResearchMissingCard(titleKey: "research.ghidra.title")
            }
        }
    }
}

// MARK: 5. SPTM register inventory

struct ResearchSPTMView: View {
    var body: some View {
        ResearchScreen(titleKey: "research.sptm.title") { catalog in
            let c = catalog.census
            SectionCard(titleKey: "research.sptm.summary") {
                LabNote(text: c.sourceDescription)
                InfoRow(labelKey: "research.sptm.registers", value: "\(c.registerCount)")
                InfoRow(labelKey: "research.sptm.access_rows", value: "\(c.accessRows)")
                InfoRow(labelKey: "research.sptm.defined", value: "\(c.definedInForkCount)")
                InfoRow(labelKey: "research.sptm.undefined", value: "\(c.undefinedInForkCount)")
                InfoRow(labelKey: "research.sptm.runtime_reachable", value: "\(c.runtimeReachableRegisterCount)")
            }
            SectionCard(titleKey: "research.sptm.registers") {
                ForEach(c.registers) { r in
                    VStack(alignment: .leading, spacing: 2) {
                        Text(verbatim: r.encoding).font(.subheadline.monospaced()).foregroundStyle(Theme.textPrimary)
                        Text(LocalizedStringKey(r.definedInFork ? "research.sptm.defined_yes" : "research.sptm.defined_no"))
                            .font(.footnote).foregroundStyle(r.definedInFork ? Theme.positive : Theme.neutral)
                        Text("\(r.accesses) · MRS \(r.mrs) · MSR \(r.msr)")
                            .font(.footnote).foregroundStyle(Theme.textSecondary)
                    }
                    .padding(.vertical, 4)
                    .accessibilityElement(children: .combine)
                }
            }
        }
    }
}

// MARK: 6. AIC and PMGR research

struct ResearchAICPMGRView: View {
    var body: some View {
        ResearchScreen(titleKey: "research.aicpmgr.title") { catalog in
            if let dt = catalog.analysis.devicetree, let aic = dt.aic {
                SectionCard(titleKey: "research.aicpmgr.aic") {
                    InfoRow(labelKey: "research.devicetree.compatible", value: aic.compatible.joined(separator: ", "))
                    InfoRow(labelKey: "research.aicpmgr.cells", value: aic.interruptCells.map { "\($0)" } ?? "-")
                    ForEach(Array(aic.reg.enumerated()), id: \.offset) { _, r in
                        InfoRow(labelKey: "research.aicpmgr.reg", value: "\(r.addressHex) · \(r.sizeHex)")
                    }
                    InfoRow(labelKey: "research.aicpmgr.translated", value: aic.translatedBaseHex ?? String(localized: "research.aicpmgr.not_established"))
                    InfoRow(labelKey: "research.aicpmgr.reg_names", value: ResearchText.yesNo(aic.regNamesPresent))
                    InfoRow(labelKey: "research.aicpmgr.iack", value: ResearchText.yesNo(aic.aicIackOffsetInTree))
                    InfoRow(labelKey: "research.aicpmgr.consumers", value: "\(aic.consumersSingleCell) / \(aic.consumersMultiElement)")
                }
                if let pmgr = dt.pmgr {
                    SectionCard(titleKey: "research.aicpmgr.pmgr") {
                        InfoRow(labelKey: "research.devicetree.compatible", value: pmgr.compatible.joined(separator: ", "))
                        InfoRow(labelKey: "research.aicpmgr.windows", value: "\(pmgr.regWindows)")
                        InfoRow(labelKey: "research.aicpmgr.domains", value: "\(pmgr.powerDomainNames.count)")
                        LabNote(text: pmgr.powerDomainNames.joined(separator: ", "))
                    }
                }
                SectionCard(titleKey: "research.aicpmgr.verdicts") {
                    ForEach(catalog.matrix.components.filter { ["interrupt_controller", "pmgr", "timer"].contains($0.id) }, id: \.id) { c in
                        VStack(alignment: .leading, spacing: 4) {
                            Text(verbatim: c.title).font(.subheadline.weight(.semibold)).foregroundStyle(Theme.textPrimary)
                            ResearchBadge(textKey: c.status.localizationKey, color: ResearchText.color(c.status), symbol: ResearchText.symbol(c.status))
                            LabNote(text: c.basis)
                        }
                        .padding(.vertical, 4)
                    }
                }
            } else {
                ResearchMissingCard(titleKey: "research.aicpmgr.title")
            }
        }
    }
}

// MARK: 7. VM feasibility

struct ResearchVMView: View {
    var body: some View {
        ResearchScreen(titleKey: "research.vm.title") { catalog in
            SectionCard(titleKey: "research.vm.exact") {
                ResearchBadge(textKey: "research.vm.not_established", color: Theme.negative, symbol: "xmark.octagon")
                LabNote(text: catalog.matrix.exactTargetVm.statement)
            }
            ForEach(catalog.matrix.components) { c in
                SectionCard(verbatimTitle: c.title) {
                    ResearchBadge(textKey: c.status.localizationKey, color: ResearchText.color(c.status), symbol: ResearchText.symbol(c.status))
                    InfoRow(labelKey: "research.vm.question", value: c.question)
                    LabNote(text: c.basis)
                }
            }
        }
    }
}

// MARK: 8. Experiment manager

struct ResearchExperimentsView: View {
    @EnvironmentObject var model: AppModel

    var body: some View {
        Screen(titleKey: "research.experiments.title") {
            DataKindBanner(textKey: "research.experiments.banner", symbol: "flask")
            if let lab = model.lab {
                if lab.experiments.isEmpty {
                    SectionCard(titleKey: "research.experiments.none") {
                        LabNote(text: String(localized: "research.experiments.none_note"))
                    }
                } else {
                    ForEach(lab.experiments) { e in
                        SectionCard(verbatimTitle: e.experimentId) {
                            LabStatusBadge.experiment(e.status)
                            InfoRow(labelKey: "research.experiments.kind", value: e.kind)
                            InfoRow(labelKey: "research.experiments.timestamp", value: e.timestamp)
                        }
                    }
                }
            } else {
                LabUnavailableView()
            }
        }
    }
}

// MARK: 9. Compatibility dashboard

struct ResearchCompatibilityView: View {
    var body: some View {
        ResearchScreen(titleKey: "research.compatibility.title") { catalog in
            SectionCard(titleKey: "research.compatibility.exact") {
                ResearchBadge(textKey: "research.vm.not_established", color: Theme.negative, symbol: "xmark.octagon")
                LabNote(text: String(localized: "research.compatibility.exact_note"))
            }
            SectionCard(titleKey: "research.status_counts") {
                ForEach(catalog.statusCounts, id: \.0) { status, count in
                    HStack {
                        ResearchBadge(textKey: status.localizationKey, color: ResearchText.color(status), symbol: ResearchText.symbol(status))
                        Spacer(minLength: 8)
                        Text("\(count)").foregroundStyle(Theme.textPrimary).monospacedDigit()
                    }
                    .frame(minHeight: 44)
                }
            }
            SectionCard(titleKey: "research.compatibility.blocked") {
                ForEach(catalog.matrix.components.filter { $0.status != .verified && $0.status != .partial }, id: \.id) { c in
                    InfoRow(labelKey: "research.compatibility.row", value: "\(c.title): \(Bundle.main.localizedString(forKey: c.status.localizationKey, value: c.status.rawValue, table: nil))")
                }
            }
        }
    }
}

// MARK: 10. Evidence explorer

struct ResearchEvidenceView: View {
    @State private var filter: ResearchConfidence?

    var body: some View {
        ResearchScreen(titleKey: "research.evidence.title") { catalog in
            DataKindBanner(textKey: "research.evidence.banner", symbol: "list.bullet.rectangle")
            SectionCard(titleKey: "research.evidence.filter") {
                Picker("research.evidence.filter", selection: $filter) {
                    Text("research.evidence.all").tag(ResearchConfidence?.none)
                    ForEach(ResearchConfidence.allCases, id: \.self) { c in
                        Text(LocalizedStringKey("research.confidence.\(c.rawValue)")).tag(ResearchConfidence?.some(c))
                    }
                }
                .pickerStyle(.menu)
            }
            SectionCard(titleKey: "research.evidence.claims") {
                ForEach(catalog.analysis.claims.filter { filter == nil || $0.confidence == filter }) { claim in
                    VStack(alignment: .leading, spacing: 4) {
                        Text(verbatim: "[\(claim.confidence.tag)] \(claim.id)")
                            .font(.footnote.monospaced()).foregroundStyle(Theme.accent)
                        Text(verbatim: claim.statement).font(.subheadline).foregroundStyle(Theme.textPrimary)
                        LabNote(text: String(localized: "research.evidence.source") + ": " + claim.source)
                        LabNote(text: String(localized: "research.evidence.limitation") + ": " + claim.limitation)
                    }
                    .padding(.vertical, 6)
                    .accessibilityElement(children: .combine)
                }
            }
            SectionCard(titleKey: "research.evidence.decisions") {
                ForEach(catalog.matrix.components, id: \.id) { c in
                    ForEach(Array(c.decisions.enumerated()), id: \.offset) { _, d in
                        VStack(alignment: .leading, spacing: 4) {
                            Text(verbatim: "[\(d.confidence.tag)] \(c.title)")
                                .font(.footnote.monospaced()).foregroundStyle(Theme.accent)
                            Text(verbatim: d.statement).font(.subheadline).foregroundStyle(Theme.textPrimary)
                            LabNote(text: String(localized: "research.evidence.source") + ": " + d.source + " · " + d.evidenceType)
                            LabNote(text: String(localized: "research.evidence.limitation") + ": " + d.limitation)
                        }
                        .padding(.vertical, 6)
                        .accessibilityElement(children: .combine)
                    }
                }
            }
        }
    }
}

/// Shown when a screen's source record is absent from a valid analysis. Nothing is filled in.
struct ResearchMissingCard: View {
    let titleKey: String

    var body: some View {
        SectionCard(titleKey: titleKey) {
            ResearchBadge(textKey: "research.missing.record", color: Theme.neutral, symbol: "tray")
            LabNote(text: String(localized: "research.missing.detail"))
        }
    }
}
