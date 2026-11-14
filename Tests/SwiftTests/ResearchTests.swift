import XCTest
@testable import HomeiKit

/// Rust/Swift parity: these tests decode the same committed files that `Lab/src/analysis` and
/// `Lab/src/feasibility.rs` produce, and assert the same figures the Rust tests assert.
final class ResearchTests: XCTestCase {
    private func data(_ name: String) throws -> Data {
        let url = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()  // Tests/SwiftTests
            .deletingLastPathComponent()  // Tests
            .deletingLastPathComponent()  // repository root
            .appendingPathComponent("Data/research")
            .appendingPathComponent(name)
        return try Data(contentsOf: url)
    }

    private func analysis() throws -> ResearchAnalysis {
        try ResearchStore.decodeAnalysis(try data("t8130-firmware-analysis.json"))
    }

    func testCommittedAnalysisDescribesTheFixedTargetAndPassesEveryCheck() throws {
        let a = try analysis()
        XCTAssertEqual(a.target.productType, "iPhone16,2")
        XCTAssertEqual(a.target.boardConfig, "D84AP")
        XCTAssertEqual(a.target.chip, "T8130")
        XCTAssertEqual(a.target.build, "24A446")
        XCTAssertTrue(a.identityChecks.allSatisfy(\.passed))
        XCTAssertNil(ResearchStore.violation(a))
    }

    func testComponentCountsMatchTheRustRun() throws {
        let s = try analysis().summary
        XCTAssertEqual(s.eraseComponents, 97)
        XCTAssertEqual(s.analysed, 28)
        XCTAssertEqual(s.encrypted, 4)
        XCTAssertEqual(s.unavailable, 61)
        XCTAssertEqual(s.noManifestPath, 4)
        XCTAssertEqual(s.missing, 0, "no component is reported missing from the archive")
        XCTAssertEqual(s.total, 97)
    }

    func testEncryptedComponentsAreExactlyTheKeybagImages() throws {
        let a = try analysis()
        let keybag = a.components.filter { $0.container?.keybagPresent == true }.map(\.name)
        let encrypted = a.components.filter { $0.availability == .encrypted }.map(\.name)
        XCTAssertEqual(Set(keybag), Set(encrypted))
        XCTAssertEqual(encrypted.count, 4)
    }

    func testDeviceTreeAndAICTranslationAreRecordedAsInferred() throws {
        let dt = try XCTUnwrap(try analysis().devicetree)
        XCTAssertEqual(dt.nodeCount, 379)
        XCTAssertEqual(dt.rootCompatible, ["D84AP", "iPhone16,2", "AppleARM"])
        let aic = try XCTUnwrap(dt.aic)
        XCTAssertEqual(aic.compatible, ["aic,3"])
        XCTAssertEqual(aic.interruptCells, 1)
        XCTAssertEqual(aic.translatedBaseHex, "0x2d1000000")
        XCTAssertEqual(aic.reg.first?.addressHex, "0xc1000000")
        XCTAssertFalse(aic.regNamesPresent)
        XCTAssertEqual(try XCTUnwrap(dt.pmgr).powerDomainNames.count, 39)
        let inferred = try analysis().claims.filter { $0.statement.contains("translates") }
        XCTAssertEqual(inferred.map(\.confidence), [.inferred])
    }

    func testKernelcacheHeaderParsesAsArm64eFileset() throws {
        let k = try XCTUnwrap(try analysis().kernelcache)
        XCTAssertNil(k.machoError)
        let m = try XCTUnwrap(k.macho)
        XCTAssertEqual(m.architecture, "arm64e")
        XCTAssertEqual(m.filetype, "MH_FILESET")
        XCTAssertEqual(k.filesetEntryCount, 264)
        XCTAssertEqual(k.xnuVersion, "xnu-13432.2.10~2")
        XCTAssertEqual(k.configurationToken, "RELEASE_ARM64_T8122")
    }

    func testGhidraMetadataReportsUnknownFunctionCounts() throws {
        let g = try XCTUnwrap(try analysis().ghidra)
        XCTAssertEqual(g.projectFileState, "empty")
        XCTAssertNil(g.functionCount)
        XCTAssertFalse(g.exportPresent)
        XCTAssertEqual(g.names.filter(\.presentInIndex).map(\.name).sorted(),
                       ["AppleInterruptControllerV3", "AppleT8130PMGR"])
    }

    func testFeasibilityMatrixHasSixteenComponentsAndNoVerifiedExactTarget() throws {
        let m = try ResearchStore.decodeMatrix(try data("t8130-feasibility.json"))
        XCTAssertEqual(m.components.count, 16)
        XCTAssertEqual(m.exactTargetVm.status, "NOT_ESTABLISHED")
        XCTAssertNil(ResearchStore.violation(m))
        let counts = Dictionary(uniqueKeysWithValues: ResearchStatus.allCases.map { s in
            (s, m.components.filter { $0.status == s }.count)
        })
        XCTAssertEqual(counts[.verified], 0)
        XCTAssertEqual(counts.values.reduce(0, +), 16)
        XCTAssertEqual(m.components.first { $0.id == "sep" }?.status, .blocked)
        XCTAssertEqual(m.components.first { $0.id == "cpu" }?.status, .unsupported)
    }

    func testCensusMatchesTheRecordedRegisterCounts() throws {
        let c = try ResearchStore.decodeCensus(try data("t8130-sptm-census.json"))
        XCTAssertEqual(c.registerCount, 127)
        XCTAssertEqual(c.undefinedInForkCount, 98)
        XCTAssertEqual(c.accessRows, 1444)
        XCTAssertNil(ResearchStore.violation(c))
    }

    func testCatalogLoadsWhenAllThreeFilesAreValid() throws {
        let result = ResearchCatalog.build(
            analysis: try data("t8130-firmware-analysis.json"),
            matrix: try data("t8130-feasibility.json"),
            census: try data("t8130-sptm-census.json"))
        guard case .success(let catalog) = result else { return XCTFail("expected a valid catalog") }
        XCTAssertEqual(catalog.statusCounts.map(\.1).reduce(0, +), 16)
    }

    func testCatalogRefusesAnAnalysisForAnotherDevice() throws {
        let text = String(decoding: try data("t8130-firmware-analysis.json"), as: UTF8.self)
        let changed = text.replacingOccurrences(of: "\"product_type\": \"iPhone16,2\"",
                                                with: "\"product_type\": \"iPhone99,9\"")
        XCTAssertNotEqual(changed, text, "the fixture must contain the product type")
        let result = ResearchCatalog.build(
            analysis: Data(changed.utf8),
            matrix: try data("t8130-feasibility.json"),
            census: try data("t8130-sptm-census.json"))
        guard case .failure(let error) = result, case .invalid = error else {
            return XCTFail("a different device must be an invalid state, not shown")
        }
    }

    func testCatalogReportsUnreadableInputAsAnErrorState() throws {
        let result = ResearchCatalog.build(
            analysis: Data("{}".utf8),
            matrix: try data("t8130-feasibility.json"),
            census: try data("t8130-sptm-census.json"))
        guard case .failure(.unreadable) = result else {
            return XCTFail("an empty analysis must be an unreadable state")
        }
    }

    func testVerifiedIsRejectedInTheMatrixInvariant() throws {
        var m = try ResearchStore.decodeMatrix(try data("t8130-feasibility.json"))
        let first = m.components[0]
        m = ResearchMatrix(schemaVersion: m.schemaVersion, target: m.target, generatedOn: m.generatedOn,
                           analysisAnalysedOn: m.analysisAnalysedOn,
                           exactTargetVm: m.exactTargetVm,
                           components: [ResearchFeasibilityComponent(
                               id: first.id, title: first.title, question: first.question,
                               status: .verified, basis: first.basis, decisions: first.decisions)]
                               + m.components.dropFirst())
        XCTAssertNotNil(ResearchStore.violation(m))
    }
}
