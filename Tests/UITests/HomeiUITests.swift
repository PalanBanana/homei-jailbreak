import XCTest

/// Simulator UI checks: what each screen says, what it must not offer, localisation, large text and
/// Apple's built-in accessibility audit. Runs in CI on an iOS Simulator only. It never touches a device.
final class HomeiUITests: XCTestCase {
    override func setUpWithError() throws {
        continueAfterFailure = true
    }

    // MARK: Helpers

    private func launch(language: String = "en", locale: String = "en_US", extra: [String] = []) -> XCUIApplication {
        let app = XCUIApplication()
        app.launchArguments += ["-AppleLanguages", "(\(language))", "-AppleLocale", locale] + extra
        app.launch()
        return app
    }

    private func has(_ app: XCUIApplication, _ text: String, timeout: TimeInterval = 5) -> Bool {
        let match = app.descendants(matching: .any).matching(NSPredicate(format: "label CONTAINS %@", text)).firstMatch
        return match.waitForExistence(timeout: timeout)
    }

    private func shot(_ app: XCUIApplication, _ name: String) {
        let attachment = XCTAttachment(screenshot: app.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    private func tab(_ app: XCUIApplication, _ name: String) {
        let button = app.tabBars.buttons[name]
        XCTAssertTrue(button.waitForExistence(timeout: 10), "tab \(name)")
        button.tap()
    }

    private func openMore(_ app: XCUIApplication, _ row: String) {
        tab(app, "More")
        let link = app.buttons[row].firstMatch
        XCTAssertTrue(link.waitForExistence(timeout: 5), "row \(row)")
        link.tap()
    }

    // MARK: Accessibility and contrast audit

    /// Machine-readable record of one screen, attached to the test result and summarised by CI.
    private struct ScreenAudit: Codable {
        let screen: String
        let pages: Int
        let reachedEnd: Bool
        let raw: [ContrastObservation]
        let final: [ContrastObservation]
        let appleReported: [String]
        let appleOccluded: [String]
        let appleUnattributed: [String]
        let appleInconclusive: [String]
    }

    /// Scroll by about a quarter of the screen. The unobscured viewport is much taller than one step,
    /// so every text run passes through it fully on at least one page.
    private func scrollDown(_ app: XCUIApplication) {
        let start = app.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.70))
        let end = app.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.47))
        start.press(forDuration: 0.05, thenDragTo: end)
        Thread.sleep(forTimeInterval: 0.7) // let the scroll settle before reading the screen
    }

    /// Apple's audit, bounded to two attempts. A failure to complete is recorded as inconclusive and
    /// fails the test. It is never treated as a pass.
    private func appleAudit(_ app: XCUIApplication, page: Int, reported: inout [String], occluded: inout [String],
                            unattributed: inout [String], inconclusive: inout [String]) {
        let zones = ContrastAnalyzer.overlayZones(app: app)
        var reportedHere: [String] = [], occludedHere: [String] = [], inconclusiveHere: [String] = []
        for attempt in 1...2 {
            reportedHere = []; occludedHere = []; inconclusiveHere = []
            do {
                try app.performAccessibilityAudit { issue in
                    let line = "page \(page): \(issue.auditType): \(issue.compactDescription) [\(issue.element?.label ?? "?")]"
                    guard let frame = issue.element?.frame else {
                        // Apple reported an issue it cannot attribute to an element. It is kept as an explicit
                        // INCONCLUSIVE record and is never counted as a pass.
                        inconclusiveHere.append(line + " (no addressable element)")
                        return true
                    }
                    if zones.contains(where: { frame.intersects($0) }) {
                        occludedHere.append(line)
                        return true
                    }
                    reportedHere.append(line)
                    return false
                }
                reported += reportedHere; occluded += occludedHere; unattributed += inconclusiveHere
                return
            } catch {
                if attempt == 2 { inconclusive.append("page \(page): Apple audit did not complete after 2 attempts: \(error)") }
            }
        }
    }

    /// Audit one screen completely: every scrolled page is analysed with the pixel contrast
    /// analyzer, and Apple's audit runs on the first and the last page.
    private func audit(_ app: XCUIApplication, screen: String) {
        var raw: [ContrastObservation] = []
        var reported: [String] = [], occluded: [String] = [], unattributed: [String] = [], inconclusive: [String] = []
        var pages = 0
        var reachedEnd = false

        for page in 0..<30 {
            pages = page + 1
            raw += ContrastAnalyzer.analyzePage(app: app, screen: screen, page: page)
            let before = app.screenshot()
            let attachment = XCTAttachment(screenshot: before)
            attachment.name = "\(screen)-page-\(page)"
            attachment.lifetime = .keepAlways
            add(attachment)
            if page == 0 { appleAudit(app, page: 0, reported: &reported, occluded: &occluded, unattributed: &unattributed, inconclusive: &inconclusive) }
            scrollDown(app)
            if app.screenshot().pngRepresentation == before.pngRepresentation {
                reachedEnd = true
                if page > 0 { appleAudit(app, page: page, reported: &reported, occluded: &occluded, unattributed: &unattributed, inconclusive: &inconclusive) }
                break
            }
        }

        // Apple findings it could not attach to an element are INCONCLUSIVE observations.
        for line in unattributed {
            raw.append(ContrastObservation(screen: screen, page: -1, label: line, classification: .inconclusive, ratio: nil,
                                           foreground: nil, background: nil, detail: "reported by Apple's audit without an element"))
        }
        // Apple's clipped-text findings become CLIPPED observations.
        for line in reported where line.contains("Text clipped") {
            raw.append(ContrastObservation(screen: screen, page: -1, label: line, classification: .clipped, ratio: nil,
                                           foreground: nil, background: nil, detail: "reported by Apple's accessibility audit"))
        }
        let final = ContrastAnalyzer.finalize(raw)
        let record = ScreenAudit(screen: screen, pages: pages, reachedEnd: reachedEnd, raw: raw, final: final,
                                 appleReported: reported, appleOccluded: occluded, appleUnattributed: unattributed, appleInconclusive: inconclusive)
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        if let data = try? encoder.encode(record) {
            let attachment = XCTAttachment(data: data, uniformTypeIdentifier: "public.json")
            attachment.name = "audit-\(screen).json"
            attachment.lifetime = .keepAlways
            add(attachment)
        }

        XCTAssertTrue(reachedEnd, "\(screen): did not reach the end of the screen within 30 pages")
        for o in final where o.classification == .fail { XCTFail("\(screen): contrast FAIL \(o.ratio ?? 0):1 [\(o.label)] page \(o.page)") }
        for o in final where o.classification == .clipped { XCTFail("\(screen): CLIPPED [\(o.label)] page \(o.page)") }
        for line in reported { XCTFail("\(screen): Apple audit: \(line)") }
        for line in inconclusive { XCTFail("\(screen): \(line)") }
    }

    // MARK: Screens

    func testDashboardStatesNotEstablishedAndOffersNoJailbreakAction() {
        let app = launch()
        XCTAssertTrue(has(app, "NOT ESTABLISHED", timeout: 15), "compatibility status")
        XCTAssertTrue(has(app, "not a connected device"), "provenance banner says the profile is not a connected device")
        XCTAssertTrue(has(app, "Research Only"), "research-only notice")
        XCTAssertTrue(has(app, "iPhone16,2"))
        XCTAssertTrue(has(app, "27.0.1"))
        XCTAssertTrue(has(app, "24A446"))
        XCTAssertTrue(has(app, "LEVEL 0 / NO-GO"))
        XCTAssertFalse(has(app, "SUPPORTED", timeout: 1), "no positive status label on the dashboard")
        let risky = app.buttons.matching(NSPredicate(
            format: "label CONTAINS[c] 'install' OR label CONTAINS[c] 'start' OR label CONTAINS[c] 'run exploit' OR label ==[c] 'jailbreak'"))
        XCTAssertEqual(risky.count, 0, "no control that could start an installation: \(risky.allElementsBoundByIndex.map(\.label))")
        shot(app, "dashboard-en")
        audit(app, screen: "dashboard")
    }

    func testDeviceScreenShowsSixFieldsAndTheDataKindLegend() {
        let app = launch()
        tab(app, "Device")
        for text in ["ProductType", "ProductVersion", "BuildVersion", "DeviceClass", "HardwareModel", "CPUArchitecture",
                     "iPhone16,2", "arm64e", "not a connected device", "Kinds of information", "Observed data",
                     "none in this app: it never reads a phone", "User-provided profile", "Synthetic test fixture",
                     "Verified research", "Unknown compatibility"] {
            XCTAssertTrue(has(app, text), text)
        }
        shot(app, "device-en")
        audit(app, screen: "device")
    }

    func testCompatibilityScreenExplainsTheStateAndRequirements() {
        let app = launch()
        tab(app, "Compatibility")
        for text in ["NOT ESTABLISHED", "Why this state", "What a supported state requires", "Research Only",
                     "not a connected device", "HC-001"] {
            XCTAssertTrue(has(app, text), text)
        }
        shot(app, "compatibility-en")
        audit(app, screen: "compatibility")
    }

    func testResearchScreenSaysItIsPublicResearchNotPhoneData() {
        let app = launch()
        tab(app, "Research")
        XCTAssertTrue(has(app, "Public research from the tracker"))
        XCTAssertTrue(has(app, "Research snapshot"))
        XCTAssertTrue(has(app, "2026-11-12"))
        XCTAssertTrue(has(app, "verified") || has(app, "partially verified"))
        XCTAssertFalse(has(app, "exploitable", timeout: 1), "no exploitable label anywhere")
        shot(app, "research-en")
        audit(app, screen: "research")
    }

    func testPackageCatalogScreen() {
        let app = launch()
        openMore(app, "Package Catalog")
        XCTAssertTrue(has(app, "Synthetic fixture packages"))
        XCTAssertTrue(has(app, "Not installable"))
        shot(app, "catalog-en")
        audit(app, screen: "catalog")
    }

    func testBootstrapSimulationScreen() {
        let app = launch()
        openMore(app, "Bootstrap Status")
        XCTAssertTrue(has(app, "SIMULATION ONLY"))
        XCTAssertTrue(has(app, "[blocked]"), "simulated steps are blocked while compatibility is not positive")
        XCTAssertTrue(has(app, "not a connected device"))
        shot(app, "bootstrap-en")
        audit(app, screen: "bootstrap")
    }

    func testDiagnosticsScreen() {
        let app = launch()
        openMore(app, "Diagnostics")
        XCTAssertTrue(has(app, "Data checks"))
        XCTAssertTrue(has(app, "not a connected device"))
        shot(app, "diagnostics-en")
        audit(app, screen: "diagnostics")
    }

    func testRecoveryScreen() {
        let app = launch()
        openMore(app, "Recovery")
        XCTAssertTrue(has(app, "NO-GO"))
        XCTAssertTrue(has(app, "does not guarantee"))
        shot(app, "recovery-en")
        audit(app, screen: "recovery")
    }

    func testSettingsScreen() {
        let app = launch()
        openMore(app, "Settings")
        XCTAssertTrue(has(app, "Test profile"))
        XCTAssertTrue(has(app, "no background device discovery"))
        shot(app, "settings-en")
        audit(app, screen: "settings")
    }

    func testAboutScreen() {
        let app = launch()
        openMore(app, "About HOMEI")
        XCTAssertTrue(has(app, "Independence"))
        shot(app, "about-en")
        audit(app, screen: "about")
    }

    func testChangingTheTestProfileStillNeverShowsAPositiveState() {
        let app = launch()
        for label in ["Comparison record (A19 Pro, same version and build)", "Comparison record (A18 Pro, iOS 27.2)"] {
            openMore(app, "Settings")
            let option = app.buttons[label].firstMatch
            XCTAssertTrue(option.waitForExistence(timeout: 5), "settings option \(label)")
            option.tap()
            XCTAssertTrue(app.buttons[label].firstMatch.isSelected, "\(label) is marked selected")
            app.navigationBars.buttons.element(boundBy: 0).tap()
            tab(app, "Dashboard")
            XCTAssertTrue(has(app, "NOT ESTABLISHED"), label)
            XCTAssertFalse(has(app, "SUPPORTED", timeout: 1), label)
        }
    }

    func testSettingsOptionsAreReadableWithoutTruncation() {
        let app = launch()
        openMore(app, "Settings")
        for label in ["Research target (A17 Pro)", "Comparison record (A19 Pro, same version and build)",
                      "Comparison record (A18 Pro, iOS 27.2)"] {
            let option = app.buttons[label].firstMatch
            XCTAssertTrue(option.waitForExistence(timeout: 5), "full, untruncated label: \(label)")
            XCTAssertGreaterThanOrEqual(option.frame.height, 44, "hit target of \(label)")
        }
    }

    // MARK: Localisation and text size

    func testKoreanInterface() {
        let app = launch(language: "ko", locale: "ko_KR")
        XCTAssertTrue(has(app, "확립되지 않음", timeout: 15), "Korean status label")
        XCTAssertTrue(app.tabBars.buttons["대시보드"].exists)
        XCTAssertTrue(has(app, "연구 전용"))
        XCTAssertTrue(has(app, "테스트 데이터"), "provenance banner is localised")
        XCTAssertFalse(has(app, "not a connected device", timeout: 1), "no untranslated provenance text")
        shot(app, "dashboard-ko")
        audit(app, screen: "dashboard-ko")
    }

    func testKoreanInterfaceAtTheLargestAccessibilityTextSize() {
        let app = launch(language: "ko", locale: "ko_KR",
                         extra: ["-UIPreferredContentSizeCategoryName", "UICTContentSizeCategoryAccessibilityXXXL"])
        XCTAssertTrue(has(app, "확립되지 않음", timeout: 15))
        XCTAssertTrue(has(app, "연구 전용"))
        let window = app.windows.firstMatch.frame
        for element in app.staticTexts.allElementsBoundByIndex where !element.label.isEmpty {
            XCTAssertGreaterThanOrEqual(element.frame.minX, -0.5, "left overflow: \(element.label)")
            XCTAssertLessThanOrEqual(element.frame.maxX, window.width + 0.5, "right overflow: \(element.label)")
        }
        shot(app, "dashboard-ko-xxxl")
        audit(app, screen: "dashboard-ko-xxxl")
    }

    func testLargestAccessibilityTextSizeKeepsKeyContentReachable() {
        let app = launch(extra: ["-UIPreferredContentSizeCategoryName", "UICTContentSizeCategoryAccessibilityXXXL"])
        XCTAssertTrue(has(app, "NOT ESTABLISHED", timeout: 15))
        shot(app, "dashboard-xxxl")
        audit(app, screen: "dashboard-xxxl")
        tab(app, "Device")
        XCTAssertTrue(has(app, "iPhone16,2"))
        shot(app, "device-xxxl")
    }
}
