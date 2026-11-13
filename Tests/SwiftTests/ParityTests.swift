import XCTest
@testable import HomeiKit

/// Swift side of the Rust/Swift parity suite. The Rust side is `Tests/CompatibilityTests/parity.rs`.
/// Both read `Tests/golden/parity-vectors.json`, whose expected outcomes are written by hand, so a
/// divergence in either implementation fails its own test.
final class ParityTests: XCTestCase {
    private typealias Case = [String: Any]

    private func vectors() throws -> [String: Any] {
        let url = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()   // SwiftTests
            .deletingLastPathComponent()   // Tests
            .appendingPathComponent("golden/parity-vectors.json")
        let data = try Data(contentsOf: url)
        return try XCTUnwrap(JSONSerialization.jsonObject(with: data) as? [String: Any])
    }

    private func cases(_ v: [String: Any], _ key: String) throws -> [Case] {
        try XCTUnwrap(v[key] as? [Case], "missing \(key)")
    }

    private func jsonData(_ object: Any) throws -> Data {
        try JSONSerialization.data(withJSONObject: object, options: [.fragmentsAllowed])
    }

    func testVectorFileIsPopulated() throws {
        let v = try vectors()
        for key in ["profile_validation", "identifier_like", "identifier_token", "decisions", "safety",
                    "evidence_class", "confidence", "finding_status", "compatibility_status", "provenance", "records"] {
            XCTAssertFalse(try cases(v, key).isEmpty, key)
        }
    }

    func testDeviceProfileValidationMatchesVectors() throws {
        for c in try cases(try vectors(), "profile_validation") {
            let json = try XCTUnwrap(c["json"] as? String)
            let outcome: String
            if let profile = try? JSONDecoder().decode(DeviceProfile.self, from: Data(json.utf8)) {
                do {
                    try profile.validate()
                    outcome = "valid"
                } catch let e as ProfileError {
                    outcome = "rejected_validate:\(e.field)"
                } catch {
                    outcome = "rejected_validate:?"
                }
            } else {
                outcome = "rejected_decode"
            }
            XCTAssertEqual(outcome, c["expect"] as? String, c["name"] as? String ?? "")
        }
    }

    func testIdentifierHeuristicsMatchVectors() throws {
        let v = try vectors()
        for c in try cases(v, "identifier_like") {
            let value = try XCTUnwrap(c["value"] as? String)
            XCTAssertEqual(looksLikeIdentifier(value), c["expect"] as? Bool, "\(value)")
        }
        for c in try cases(v, "identifier_token") {
            let text = try XCTUnwrap(c["text"] as? String)
            XCTAssertEqual(DiagnosticsReport.containsIdentifierLikeToken(text), c["expect"] as? Bool, "\(text)")
        }
    }

    func testCompatibilityDecisionsMatchVectors() throws {
        let decoder = JSONDecoder()
        for c in try cases(try vectors(), "decisions") {
            let name = c["name"] as? String ?? ""
            let profile = try decoder.decode(DeviceProfile.self, from: jsonData(try XCTUnwrap(c["profile"])))
            let declarations = try decoder.decode([Declaration].self, from: jsonData(try XCTUnwrap(c["declarations"])))
            let evidence = try decoder.decode([Evidence].self, from: jsonData(try XCTUnwrap(c["evidence"])))
            let d = CompatibilityEngine.evaluate(profile: profile, declarations: declarations, evidence: evidence)
            let expect = try XCTUnwrap(c["expect"] as? Case)
            XCTAssertEqual(d.status.rawValue, expect["status"] as? String, "\(name): status")
            XCTAssertEqual(d.matchedDeclaration, expect["matched"] as? String, "\(name): matched record")
            XCTAssertEqual(d.downgraded, expect["downgraded"] as? Bool, "\(name): downgraded")
            if d.status.isPositive {
                XCTAssertNotNil(d.matchedDeclaration, name)
                XCTAssertFalse(d.downgraded, name)
            }
        }
    }

    func testSafetyDecisionsMatchVectors() throws {
        for c in try cases(try vectors(), "safety") {
            let op = try XCTUnwrap(c["operation"] as? String)
            let decision = SafetyManager.authorize(operation: op)
            XCTAssertEqual(decision.isAllowed, c["allowed"] as? Bool, "\(op.debugDescription)")
            switch decision {
            case .allowed:
                XCTAssertTrue(c["reason"] is NSNull, "\(op.debugDescription)")
            case .denied(let reason):
                XCTAssertEqual(reason, c["reason"] as? String, "\(op.debugDescription): reason text")
            }
        }
    }

    private func accepts<T: Decodable>(_ type: T.Type, _ value: String) -> Bool {
        (try? JSONDecoder().decode(T.self, from: Data("\"\(value)\"".utf8))) != nil
    }

    func testClosedVocabulariesMatchVectors() throws {
        let v = try vectors()
        func check(_ key: String, _ f: (String) -> Bool) throws {
            for c in try cases(v, key) {
                let value = try XCTUnwrap(c["value"] as? String)
                XCTAssertEqual(f(value), c["valid"] as? Bool, "\(key): \(value.debugDescription)")
            }
        }
        try check("evidence_class") { self.accepts(EvidenceClass.self, $0) }
        try check("confidence") { self.accepts(Confidence.self, $0) }
        try check("finding_status") { self.accepts(FindingStatus.self, $0) }
        try check("compatibility_status") { self.accepts(CompatibilityStatus.self, $0) }
        try check("provenance") { self.accepts(Provenance.self, $0) }
    }

    func testStrictRecordDecodingMatchesVectors() throws {
        for c in try cases(try vectors(), "records") {
            let kind = try XCTUnwrap(c["kind"] as? String)
            let data = Data(try XCTUnwrap(c["json"] as? String).utf8)
            let decoder = JSONDecoder()
            let ok: Bool
            switch kind {
            case "evidence": ok = (try? decoder.decode(Evidence.self, from: data)) != nil
            case "declaration": ok = (try? decoder.decode(Declaration.self, from: data)) != nil
            case "finding": ok = (try? decoder.decode(Finding.self, from: data)) != nil
            case "fixture": ok = (try? decoder.decode(Fixture.self, from: data)) != nil
            default:
                XCTFail("unknown kind \(kind)")
                continue
            }
            XCTAssertEqual(ok, (c["expect"] as? String) == "ok", "\(kind) / \(c["name"] as? String ?? "")")
        }
    }

    /// The research target must never come out positive in the Swift implementation either.
    func testTargetFixtureIsNotEstablishedInSwift() throws {
        let dataDir = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().appendingPathComponent("Data")
        let store = try DataStore { try Data(contentsOf: dataDir.appendingPathComponent($0)) }
        let target = try XCTUnwrap(store.fixtures.first { $0.id == "FIX-A" }).profile
        XCTAssertEqual([target.productType, target.productVersion, target.buildVersion, target.cpuArchitecture ?? ""],
                       ["iPhone16,2", "27.0.1", "24A446", "arm64e"])
        XCTAssertEqual(target.socFamily, .a17Pro)
        let d = CompatibilityEngine.evaluate(profile: target, declarations: store.declarations, evidence: store.evidence)
        XCTAssertEqual(d.status, .notEstablished)
        XCTAssertEqual(d.matchedDeclaration, "HC-001")
        XCTAssertFalse(d.status.isPositive)
    }
}
