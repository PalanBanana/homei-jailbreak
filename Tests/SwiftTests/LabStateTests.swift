import XCTest
@testable import HomeiKit

/// Swift side of the lab-state parity check. The Rust side is `Tests/LabTests/state.rs`. Both read the
/// file that the host controller exported, `Tests/golden/lab-state-fixture.json`, and both decode it
/// strictly: an unknown key is an error.
final class LabStateTests: XCTestCase {
    private var repo: URL {
        URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
    }

    private func fixtureData() throws -> Data {
        try Data(contentsOf: repo.appendingPathComponent("Tests/golden/lab-state-fixture.json"))
    }

    private func fixtureObject() throws -> [String: Any] {
        try XCTUnwrap(JSONSerialization.jsonObject(with: fixtureData()) as? [String: Any])
    }

    private func data(_ object: [String: Any]) throws -> Data {
        try JSONSerialization.data(withJSONObject: object)
    }

    private func state() throws -> LabState { try LabStore.decode(fixtureData()) }

    // MARK: Decoding

    func testTheExportedStateDecodesStrictly() throws {
        let s = try state()
        XCTAssertEqual(s.target.productType, "iPhone16,2")
        XCTAssertEqual(s.target.buildVersion, "24A446")
        XCTAssertFalse(s.firmware.isEmpty)
        XCTAssertFalse(s.experiments.isEmpty)
    }

    func testTheBundledCopyIsTheSameFileAsTheFixture() throws {
        let bundled = try Data(contentsOf: repo.appendingPathComponent("Data/lab/lab-state.json"))
        XCTAssertEqual(bundled, try fixtureData())
    }

    func testAnUnknownTopLevelKeyIsRejected() throws {
        var object = try fixtureObject()
        object["udid"] = "not-a-real-identifier"
        XCTAssertThrowsError(try LabStore.decode(try data(object))) { error in
            XCTAssertEqual(error as? LabStateError, .unknownFields(["/udid"]))
        }
    }

    func testAnUnknownNestedKeyIsRejected() throws {
        var object = try fixtureObject()
        var host = try XCTUnwrap(object["host"] as? [String: Any])
        host["serial_number"] = "x"
        object["host"] = host
        XCTAssertThrowsError(try LabStore.decode(try data(object))) { error in
            XCTAssertEqual(error as? LabStateError, .unknownFields(["/host/serial_number"]))
        }
    }

    func testAnUnknownKeyInsideAListedRecordIsRejected() throws {
        var object = try fixtureObject()
        var experiments = try XCTUnwrap(object["experiments"] as? [[String: Any]])
        experiments[0]["note"] = "extra"
        object["experiments"] = experiments
        XCTAssertThrowsError(try LabStore.decode(try data(object))) { error in
            XCTAssertEqual(error as? LabStateError, .unknownFields(["/experiments[0]/note"]))
        }
    }

    func testAnUnknownStatusValueIsRejected() throws {
        var object = try fixtureObject()
        var fidelity = try XCTUnwrap(object["fidelity"] as? [String: Any])
        fidelity["vm_status"] = "EXPLOITABLE"
        object["fidelity"] = fidelity
        XCTAssertThrowsError(try LabStore.decode(try data(object)))
    }

    // MARK: Honest status

    func testTheVMStatusVocabularyIsExactlyTheSpecifiedNine() {
        XCTAssertEqual(Set(LabVmStatus.allCases.map(\.rawValue)), [
            "EXACT_TARGET_VERIFIED", "TARGET_KERNEL_BOOTED_HARDWARE_UNVERIFIED", "PARTIAL_DARWIN_GUEST",
            "REFERENCE_DEVICE_ONLY", "UNSUPPORTED_TARGET", "BLOCKED_BY_MISSING_FIRMWARE",
            "BLOCKED_BY_MISSING_QEMU_FEATURE", "BOOT_FAILURE", "NOT_TESTED",
        ])
    }

    func testOnlyTheExactTargetStatusClaimsAnExactTarget() {
        for status in LabVmStatus.allCases {
            XCTAssertEqual(status.claimsExactTarget, status == .exactTargetVerified, status.rawValue)
        }
    }

    func testTheExportedStateDoesNotClaimAnExactVM() throws {
        let s = try state()
        XCTAssertFalse(s.claimsExactTargetVM)
        XCTAssertNotEqual(s.fidelity.vmStatus, .exactTargetVerified)
        XCTAssertFalse(s.guest.vmRunning)
        XCTAssertFalse(s.hasRealBackendExperiment, "no real boot was recorded")
        XCTAssertTrue(s.fidelity.notes.contains { $0.contains("not an exact target") })
    }

    func testReferenceMetadataCannotCarryABootStatus() throws {
        var object = try fixtureObject()
        var target = try XCTUnwrap(object["target"] as? [String: Any])
        target["vm_status"] = "EXACT_TARGET_VERIFIED"
        object["target"] = target
        XCTAssertThrowsError(try LabStore.decode(try data(object))) { error in
            XCTAssertTrue("\(error)".contains("reference metadata may not carry VM status"), "\(error)")
        }
    }

    func testAnotherDeviceCannotBeSubstitutedForTheTarget() throws {
        for (field, value) in [("product_type", "iPhone17,2"), ("soc", "Apple A18 Pro"), ("build_version", "24B119"),
                               ("board", "D94AP"), ("os_version", "27.2"), ("commercial_model", "iPhone 16 Pro Max")] {
            var object = try fixtureObject()
            var target = try XCTUnwrap(object["target"] as? [String: Any])
            target[field] = value
            object["target"] = target
            XCTAssertThrowsError(try LabStore.decode(try data(object)), field) { error in
                XCTAssertTrue("\(error)".contains(field), "\(field): \(error)")
            }
        }
    }

    func testAFidelityReportForAnotherTargetIsRejected() throws {
        var object = try fixtureObject()
        var fidelity = try XCTUnwrap(object["fidelity"] as? [String: Any])
        fidelity["target_id"] = "another_target"
        object["fidelity"] = fidelity
        XCTAssertThrowsError(try LabStore.decode(try data(object)))
    }

    func testALocalPathInTheStateIsRejected() throws {
        // Built from parts so this file itself contains no path-like text.
        let markers = ["/" + "home" + "/x/analysis", "/" + "Users" + "/x/analysis", "/" + "tmp" + "/work"]
        for marker in markers {
            var object = try fixtureObject()
            var ghidra = try XCTUnwrap(object["ghidra"] as? [String: Any])
            ghidra["source_reference"] = marker
            object["ghidra"] = ghidra
            XCTAssertThrowsError(try LabStore.decode(try data(object)), marker) { error in
                XCTAssertTrue("\(error)".contains("local path marker"), "\(error)")
            }
        }
    }

    func testMissingProvenanceIsRejected() throws {
        var object = try fixtureObject()
        var target = try XCTUnwrap(object["target"] as? [String: Any])
        var provenance = try XCTUnwrap(target["provenance"] as? [String: Any])
        provenance.removeValue(forKey: "soc")
        target["provenance"] = provenance
        object["target"] = target
        XCTAssertThrowsError(try LabStore.decode(try data(object))) { error in
            XCTAssertTrue("\(error)".contains("soc has no provenance"), "\(error)")
        }
    }

    // MARK: Fidelity, experiments and guest

    func testExactAndApproximateAreDistinctStatusesWithDistinctKeys() {
        XCTAssertNotEqual(LabFidelityStatus.exact.localizationKey, LabFidelityStatus.approximate.localizationKey)
        XCTAssertEqual(Set(LabFidelityStatus.allCases.map(\.rawValue)),
                       ["EXACT", "PARTIAL", "APPROXIMATE", "UNSUPPORTED", "UNVERIFIED", "NOT_TESTED"])
    }

    func testFunctionalFeaturesAreListedOnTheirOwn() throws {
        let f = try state().fidelity
        for name in LabState.featureOrder { XCTAssertNotNil(f.functional[name], name) }
        XCTAssertEqual(f.functional["wifi"], .unsupported)
        XCTAssertEqual(f.functional["bluetooth"], .unsupported)
        XCTAssertEqual(f.functional["graphics"], .unsupported)
        XCTAssertNotEqual(f.functional["guest_boot"], .exact)
    }

    func testTheExperimentStatusVocabularyAndSuccessRule() {
        XCTAssertEqual(Set(LabExperimentStatus.allCases.map(\.rawValue)), [
            "PLANNED", "RUNNING", "COMPLETED", "FAILED", "INTERRUPTED", "INCONCLUSIVE", "NOT_REPRODUCIBLE",
        ])
        for s in LabExperimentStatus.allCases { XCTAssertEqual(s.isSuccess, s == .completed, s.rawValue) }
    }

    func testRecordedExperimentsDoNotClaimABootOrAGuestBuild() throws {
        for e in try state().experiments {
            XCTAssertNotEqual(e.backend.kind, .real, e.experimentId)
            XCTAssertNil(e.guestOsBuild, e.experimentId)
            XCTAssertNil(e.bootOutcome, e.experimentId)
            XCTAssertFalse(e.limitations.isEmpty, e.experimentId)
            XCTAssertEqual(e.target.buildVersion, "24A446")
        }
    }

    func testGuestRootNeverImpliesPhysicalDeviceRoot() {
        XCTAssertTrue(LabPrivilegeScope.guestRoot.implies(.guestRoot))
        XCTAssertFalse(LabPrivilegeScope.guestRoot.implies(.physicalDeviceRoot))
        XCTAssertFalse(LabPrivilegeScope.physicalDeviceRoot.implies(.guestRoot))
    }

    func testGuestWorkspaceIsAnIsolatedOverlayAndPackagesAreGuestOnly() throws {
        let g = try state().guest
        XCTAssertTrue(g.filesystemScope.contains("not a device"))
        XCTAssertTrue(g.overlayPresent)
        XCTAssertEqual(g.bootstrapState, "complete")
        XCTAssertEqual(g.bootstrapVersion, "1")
        for p in g.packages {
            XCTAssertFalse(p.id.isEmpty)
            XCTAssertFalse(p.version.isEmpty)
        }
    }

    // MARK: Metadata

    func testFirmwareMetadataIsForTheTargetBuildWithWellFormedHashes() throws {
        let hex = Set("0123456789abcdef")
        for f in try state().firmware {
            XCTAssertEqual(f.sourceBuild, "24A446", f.name)
            XCTAssertEqual(f.sha256.count, 64, f.name)
            XCTAssertTrue(f.sha256.allSatisfy { hex.contains($0) }, f.name)
            XCTAssertFalse(f.name.contains("/"), f.name)
        }
    }

    func testTheSymbolMatchRateIsFormattedFromIntegerBasisPoints() throws {
        var g = try state().ghidra
        g.symbolMatchBasisPoints = 7556
        XCTAssertEqual(g.symbolMatchText, "75.56%")
        g.symbolMatchBasisPoints = 5
        XCTAssertEqual(g.symbolMatchText, "0.05%")
        g.symbolMatchBasisPoints = nil
        XCTAssertNil(g.symbolMatchText)
    }

    // MARK: Localisation

    private var resources: URL { repo.appendingPathComponent("App/Resources") }

    private func bundle(_ language: String) throws -> Bundle {
        try XCTUnwrap(Bundle(path: resources.appendingPathComponent("\(language).lproj").path), language)
    }

    private func hasHangul(_ s: String) -> Bool { s.unicodeScalars.contains { (0xAC00...0xD7A3).contains($0.value) } }

    private var allLabKeys: [String] {
        var keys: [String] = []
        for s in LabVmStatus.allCases { keys += [s.localizationKey, s.detailLocalizationKey] }
        keys += LabHardwareFidelity.allCases.map(\.localizationKey)
        keys += LabFidelityStatus.allCases.map(\.localizationKey)
        keys += LabFirmwareIdentity.allCases.map(\.localizationKey)
        keys += LabBootOutcome.allCases.map(\.localizationKey)
        keys += LabExperimentStatus.allCases.map(\.localizationKey)
        keys += LabBackendKind.allCases.map(\.localizationKey)
        keys += LabState.featureOrder.map { "lab.feature.\($0)" }
        return keys
    }

    func testEveryLabStatusHasAnEnglishAndAKoreanString() throws {
        let en = try bundle("en"), ko = try bundle("ko")
        // Names that are the same in both languages.
        let sameInBoth: Set<String> = ["lab.feature.springboard", "lab.feature.wifi"]
        for key in allLabKeys {
            let english = en.localizedString(forKey: key, value: "MISSING", table: nil)
            let korean = ko.localizedString(forKey: key, value: "MISSING", table: nil)
            XCTAssertNotEqual(english, "MISSING", "en is missing \(key)")
            XCTAssertNotEqual(korean, "MISSING", "ko is missing \(key)")
            if !sameInBoth.contains(key) {
                XCTAssertNotEqual(english, korean, "\(key) is not translated")
                XCTAssertTrue(hasHangul(korean), "\(key): \(korean)")
            }
        }
    }

    func testNoLabStatusStringClaimsAnExactVMExceptTheExactStatus() throws {
        for language in ["en", "ko"] {
            let b = try bundle(language)
            for s in LabVmStatus.allCases where s != .exactTargetVerified {
                for key in [s.localizationKey, s.detailLocalizationKey] {
                    let text = b.localizedString(forKey: key, value: "", table: nil)
                    XCTAssertFalse(text.uppercased().contains("EXACT TARGET VERIFIED"), "\(language) \(key)")
                    XCTAssertFalse(text.contains("정확한 대상 검증됨"), "\(language) \(key)")
                }
            }
        }
    }
}
