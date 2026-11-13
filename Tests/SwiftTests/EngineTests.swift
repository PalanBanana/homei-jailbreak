import XCTest
@testable import HomeiKit

/// Mirrors the Rust compatibility and safety tests. Not compiled in the Linux development
/// environment: run with `swift test` on a machine with a Swift toolchain.
final class EngineTests: XCTestCase {
    private func store() throws -> DataStore {
        let dataDir = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().appendingPathComponent("Data")
        return try DataStore { try Data(contentsOf: dataDir.appendingPathComponent($0)) }
    }

    func testEveryBundledFixtureIsNotEstablished() throws {
        let s = try store()
        XCTAssertEqual(s.fixtures.map(\.id), ["FIX-A", "FIX-B", "FIX-C"])
        for f in s.fixtures {
            let d = CompatibilityEngine.evaluate(profile: f.profile, declarations: s.declarations, evidence: s.evidence)
            XCTAssertEqual(d.status, .notEstablished, f.id)
            XCTAssertFalse(d.status.isPositive)
            XCTAssertEqual(d.provenance, .testFixture)
        }
    }

    func testPositiveDeclarationWithoutExactEvidenceIsDowngraded() throws {
        let s = try store()
        var decl = s.declarations[0]
        decl.status = .supported
        let target = s.fixtures[0].profile
        let d = CompatibilityEngine.evaluate(profile: target, declarations: [decl], evidence: s.evidence)
        XCTAssertEqual(d.status, .notEstablished)
        XCTAssertTrue(d.downgraded)
    }

    func testSupportedNeedsExactTargetResultAndReproduction() throws {
        let s = try store()
        let target = s.fixtures[0].profile
        var decl = s.declarations[0]
        decl.status = .supported
        decl.evidenceIds = ["T", "R"]
        let scope = EvidenceScope(productType: "iPhone16,2", productVersion: "27.0.1", buildVersion: "24A446")
        let t = Evidence(id: "T", evidenceClass: .targetCompatibleResult, scope: scope, confidence: .medium)
        let r = Evidence(id: "R", evidenceClass: .independentReproduction, scope: scope, confidence: .medium)
        XCTAssertEqual(CompatibilityEngine.evaluate(profile: target, declarations: [decl], evidence: [t, r]).status, .supported)
        XCTAssertEqual(CompatibilityEngine.evaluate(profile: target, declarations: [decl], evidence: [t]).status, .notEstablished)
        var wrongBuild = scope; wrongBuild.buildVersion = "24A447"
        let t2 = Evidence(id: "T", evidenceClass: .targetCompatibleResult, scope: wrongBuild, confidence: .high)
        let r2 = Evidence(id: "R", evidenceClass: .independentReproduction, scope: wrongBuild, confidence: .high)
        XCTAssertEqual(CompatibilityEngine.evaluate(profile: target, declarations: [decl], evidence: [t2, r2]).status, .notEstablished)
    }

    func testProfilesRejectIdentifiersAndUnknownKeys() throws {
        let json = #"{"product_type":"iPhone16,2","product_version":"27.0.1","build_version":"24A446","serial_number":"x","provenance":"test_fixture"}"#
        XCTAssertThrowsError(try JSONDecoder().decode(DeviceProfile.self, from: Data(json.utf8)))
        var p = DeviceProfile(productType: "iPhone16,2", productVersion: "27.0.1", buildVersion: "24A446", provenance: .testFixture)
        XCTAssertNoThrow(try p.validate())
        p.hardwareModel = "0123456789abcdef0123456789abcdef01234567"
        XCTAssertThrowsError(try p.validate())
    }

    func testSafetyManagerAllowsOnlyReadOnlyOperations() {
        for op in SafetyManager.readOnlyOperations { XCTAssertTrue(SafetyManager.authorize(operation: op).isAllowed) }
        for op in ["install_jailbreak", "execute_exploit", "pair_device", "run_remote_script", "", "frobnicate"] {
            XCTAssertFalse(SafetyManager.authorize(operation: op).isAllowed, op)
        }
        XCTAssertEqual(SafetyManager.privilegedCapabilities.count, 12)
        XCTAssertFalse(CompatibilityEngine.jailbreakActionAvailable)
    }
}
