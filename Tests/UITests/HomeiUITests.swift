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

    /// Run Apple's accessibility audit on the top of the screen and on every scrolled page, so content
    /// that starts out under the floating tab bar is also measured while it is unobscured.
    ///
    /// An issue is reported unless its element physically overlaps the system tab bar or navigation
    /// bar. Those are recorded separately as "obscured", never silently dropped, because the audit
    /// measures text against the translucent bar behind it and not against the card it belongs to.
    private func audit(_ app: XCUIApplication, screen: String) {
        var reported: [String] = []
        var obscured: [String] = []
        var pages = 0

        func runOnce(page: Int) {
            let bar = app.tabBars.firstMatch.frame
            let nav = app.navigationBars.firstMatch.frame
            var attempt = 0
            while true {
                attempt += 1
                do {
                    try app.performAccessibilityAudit { issue in
                        let line = "page \(page): \(issue.auditType): \(issue.compactDescription) [\(issue.element?.label ?? "?")]"
                        if let frame = issue.element?.frame, frame.intersects(bar) || frame.intersects(nav) {
                            obscured.append(line)
                            return true
                        }
                        reported.append(line)
                        return false
                    }
                    return
                } catch {
                    if attempt >= 2 {
                        reported.append("page \(page): audit could not run: \(error)")
                        return
                    }
                }
            }
        }

        runOnce(page: 0)
        var previous = app.screenshot().pngRepresentation
        for page in 1...5 {
            app.swipeUp()
            let current = app.screenshot().pngRepresentation
            if current == previous { break }
            previous = current
            pages = page
            runOnce(page: page)
        }

        let summary = "audited \(pages + 1) page(s)\nreported:\n" + (reported.isEmpty ? "  none" : reported.map { "  " + $0 }.joined(separator: "\n"))
            + "\nobscured by system bars (not counted):\n" + (obscured.isEmpty ? "  none" : obscured.map { "  " + $0 }.joined(separator: "\n"))
        let attachment = XCTAttachment(string: summary)
        attachment.name = "accessibility-audit-\(screen)"
        attachment.lifetime = .keepAlways
        add(attachment)
        for issue in reported { XCTFail("\(screen): \(issue)") }
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
            XCTAssertTrue(option.waitForExistence(timeout: 5), label)
            option.tap()
            app.navigationBars.buttons.element(boundBy: 0).tap()
            tab(app, "Dashboard")
            XCTAssertTrue(has(app, "NOT ESTABLISHED"), label)
            XCTAssertFalse(has(app, "SUPPORTED", timeout: 1), label)
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
