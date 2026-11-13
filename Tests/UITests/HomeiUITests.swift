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

    private func back(_ app: XCUIApplication) {
        app.navigationBars.buttons.element(boundBy: 0).tap()
    }

    /// Run Apple's accessibility audit and fail on every issue it reports, keeping a readable record.
    private func audit(_ app: XCUIApplication, screen: String) {
        var found: [String] = []
        do {
            try app.performAccessibilityAudit { issue in
                found.append("\(issue.auditType): \(issue.compactDescription) [\(issue.element?.label ?? "?")]")
                return false
            }
        } catch {
            XCTFail("\(screen): audit could not run: \(error)")
        }
        let attachment = XCTAttachment(string: found.isEmpty ? "no issues" : found.joined(separator: "\n"))
        attachment.name = "accessibility-audit-\(screen)"
        attachment.lifetime = .keepAlways
        add(attachment)
        print("A11Y-AUDIT \(screen): \(found.isEmpty ? "no issues" : "\(found.count) issue(s)")")
        for line in found { print("A11Y-ISSUE \(screen): \(line)") }
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

    func testMoreScreensCatalogBootstrapDiagnosticsRecoverySettingsAbout() {
        let app = launch()

        openMore(app, "Package Catalog")
        XCTAssertTrue(has(app, "Synthetic fixture packages"))
        XCTAssertTrue(has(app, "Not installable"))
        shot(app, "catalog-en")
        audit(app, screen: "catalog")
        back(app)

        openMore(app, "Bootstrap Status")
        XCTAssertTrue(has(app, "SIMULATION ONLY"))
        XCTAssertTrue(has(app, "[blocked]"), "simulated steps are blocked while compatibility is not positive")
        shot(app, "bootstrap-en")
        audit(app, screen: "bootstrap")
        back(app)

        openMore(app, "Diagnostics")
        XCTAssertTrue(has(app, "Data checks"))
        XCTAssertTrue(has(app, "not a connected device"))
        shot(app, "diagnostics-en")
        audit(app, screen: "diagnostics")
        back(app)

        openMore(app, "Recovery")
        XCTAssertTrue(has(app, "NO-GO"))
        XCTAssertTrue(has(app, "does not guarantee"))
        shot(app, "recovery-en")
        audit(app, screen: "recovery")
        back(app)

        openMore(app, "Settings")
        XCTAssertTrue(has(app, "Test profile"))
        XCTAssertTrue(has(app, "no background device discovery"))
        shot(app, "settings-en")
        audit(app, screen: "settings")
        back(app)

        openMore(app, "About HOMEI")
        XCTAssertTrue(has(app, "Independence"))
        shot(app, "about-en")
        audit(app, screen: "about")
    }

    func testChangingTheTestProfileStillNeverShowsAPositiveState() {
        let app = launch()
        openMore(app, "Settings")
        for label in ["Comparison record (A19 Pro, same version and build)", "Comparison record (A18 Pro, iOS 27.2)"] {
            let option = app.buttons[label].firstMatch
            if option.waitForExistence(timeout: 5) { option.tap() }
            back(app)
            tab(app, "Dashboard")
            XCTAssertTrue(has(app, "NOT ESTABLISHED"), label)
            openMore(app, "Settings")
        }
    }

    // MARK: Localisation and text size

    func testKoreanInterface() {
        let app = launch(language: "ko", locale: "ko_KR")
        XCTAssertTrue(has(app, "확립되지 않음", timeout: 15), "Korean status label")
        XCTAssertTrue(app.tabBars.buttons["대시보드"].exists)
        XCTAssertTrue(has(app, "연구 전용"))
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
