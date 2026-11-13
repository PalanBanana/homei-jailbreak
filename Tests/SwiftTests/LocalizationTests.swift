import XCTest
@testable import HomeiKit

/// Resource-level localisation checks that run without a simulator. They read the real
/// Localizable.strings files the app ships and look keys up through Bundle, the same mechanism the
/// app uses. Rendering is checked separately by the simulator UI tests.
final class LocalizationTests: XCTestCase {
    private var resources: URL {
        URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().appendingPathComponent("App/Resources")
    }

    private func bundle(_ language: String) throws -> Bundle {
        try XCTUnwrap(Bundle(path: resources.appendingPathComponent("\(language).lproj").path), language)
    }

    private func hasHangul(_ s: String) -> Bool {
        s.unicodeScalars.contains { (0xAC00...0xD7A3).contains($0.value) }
    }

    func testEveryProvenanceHasAnEnglishLabelEqualToTheKitLabel() throws {
        let en = try bundle("en")
        for p in Provenance.allCases {
            let text = en.localizedString(forKey: p.localizationKey, value: "MISSING", table: nil)
            XCTAssertNotEqual(text, "MISSING", "en is missing \(p.localizationKey)")
            XCTAssertEqual(text, p.label, "English string and Kit label must be identical for \(p)")
        }
    }

    func testEveryProvenanceHasADistinctKoreanLabel() throws {
        let en = try bundle("en"), ko = try bundle("ko")
        var seen = Set<String>()
        for p in Provenance.allCases {
            let korean = ko.localizedString(forKey: p.localizationKey, value: "MISSING", table: nil)
            XCTAssertNotEqual(korean, "MISSING", "ko is missing \(p.localizationKey)")
            XCTAssertNotEqual(korean, en.localizedString(forKey: p.localizationKey, value: "", table: nil), "\(p) is not translated")
            XCTAssertTrue(hasHangul(korean), "\(p): \(korean)")
            XCTAssertTrue(seen.insert(korean).inserted, "two provenance kinds share one Korean label")
        }
    }

    func testNoProvenanceLabelClaimsALiveConnection() throws {
        for language in ["en", "ko"] {
            let b = try bundle(language)
            for p in Provenance.allCases.map({ Optional($0) }) + [nil] {
                let key = p?.localizationKey ?? "provenance.unknown"
                let text = b.localizedString(forKey: key, value: "", table: nil).lowercased()
                XCTAssertFalse(text.contains("live connection") || text.contains("connected to"), "\(language) \(key): \(text)")
            }
        }
    }

    func testMissingKeyFallsBackToASafeLabelInsteadOfARawKey() throws {
        for language in ["en", "ko"] {
            let b = try bundle(language)
            let fallback = b.localizedString(forKey: "provenance.unknown", value: "MISSING", table: nil)
            XCTAssertNotEqual(fallback, "MISSING", language)
            // What the banner does for a key that does not exist.
            let shown = b.localizedString(forKey: "provenance.some_future_kind", value: fallback, table: nil)
            XCTAssertEqual(shown, fallback, language)
            XCTAssertFalse(shown.hasPrefix("provenance."), "raw key must never be displayed")
        }
    }

    func testEnglishAndKoreanKeySetsAreIdentical() throws {
        func keys(_ language: String) throws -> Set<String> {
            let text = try String(contentsOf: resources.appendingPathComponent("\(language).lproj/Localizable.strings"), encoding: .utf8)
            return Set(text.split(separator: "\n").compactMap { line in
                guard line.hasPrefix("\""), let end = line.dropFirst().firstIndex(of: "\"") else { return nil }
                return String(line[line.index(after: line.startIndex)..<end])
            })
        }
        let en = try keys("en"), ko = try keys("ko")
        XCTAssertFalse(en.isEmpty)
        XCTAssertEqual(en.subtracting(ko), [], "missing in ko")
        XCTAssertEqual(ko.subtracting(en), [], "missing in en")
    }
}
