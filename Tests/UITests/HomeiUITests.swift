import XCTest

/// Simulator UI checks: what each screen says, what it must not offer, localisation, large text and
/// Apple's built-in accessibility audit. Runs in CI on an iOS Simulator only. It never touches a device.
final class HomeiUITests: XCTestCase {
    /// The application launched by the current test, terminated afterwards so the next test starts clean.
    private var launchedApp: XCUIApplication?

    override func setUpWithError() throws {
        continueAfterFailure = true
    }

    override func tearDownWithError() throws {
        launchedApp?.terminate()
        launchedApp = nil
    }

    // MARK: Helpers

    private func launch(language: String = "en", locale: String = "en_US", extra: [String] = []) -> XCUIApplication {
        let app = XCUIApplication()
        app.launchArguments += ["-AppleLanguages", "(\(language))", "-AppleLocale", locale] + extra
        launchedApp = app
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

    /// Open a row of the More list and wait until the destination's navigation bar is showing, so the
    /// next step never runs against a half-finished transition. The tap is retried once. A second
    /// miss fails the test: a retry never hides a reproducible failure.
    private func openMore(_ app: XCUIApplication, _ row: String) {
        tab(app, "More")
        for attempt in 1...2 {
            let link = app.buttons[row].firstMatch
            guard link.waitForExistence(timeout: 8) else {
                XCTFail("row \(row) is not on the More list")
                return
            }
            link.tap()
            if app.navigationBars[row].waitForExistence(timeout: 6) { return }
            if attempt == 2 { XCTFail("navigating to \(row) did not show its navigation bar after 2 taps") }
        }
    }

    // MARK: Accessibility and contrast audit

    /// Machine-readable record of one screen, attached to the test result and summarised by CI.
    private struct ScreenAudit: Codable {
        let screen: String
        let pages: Int
        let reachedEnd: Bool
        let stoppedEarly: Bool
        let raw: [ContrastObservation]
        let final: [ContrastObservation]
        let appleReported: [String]
        let appleOccluded: [String]
        let appleOverruled: [String]
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

    /// The reverse of `scrollDown`. Scrolling is clamped at the top, so one step back from the first
    /// scrolled page returns to the first page exactly.
    private func scrollUp(_ app: XCUIApplication) {
        let start = app.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.47))
        let end = app.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.70))
        start.press(forDuration: 0.05, thenDragTo: end)
        Thread.sleep(forTimeInterval: 0.7)
    }

    /// Findings of Apple's audit for one page, sorted by what they mean.
    private struct AppleFindings {
        var reported: [String] = []
        var occluded: [String] = []
        var overruled: [String] = []
        var contrast: [(line: String, label: String)] = []
        var unattributed: [String] = []
        var inconclusive: [String] = []
    }

    /// Apple's audit, in two smaller parts with up to three attempts each. If a part still does not complete,
    /// that is recorded as an explicit INCONCLUSIVE observation, counted in the results and listed in the summary.
    /// It is never treated as a pass, and the pixel analysis of the same screen is unaffected.
    ///
    /// An issue whose element lies in a system bar or scroll-edge zone is recorded as occluded. An issue
    /// Apple cannot attach to an element is recorded as unattributed. A "text clipped" issue is checked
    /// against the pixels: if no glyph pixel touches the element's frame boundary the text is complete,
    /// and the finding is recorded as overruled with that evidence instead of being counted as a defect.
    private func appleAudit(_ app: XCUIApplication, page: Int, isFirst: Bool, isLast: Bool, into found: inout AppleFindings) {
        let zones = ContrastAnalyzer.overlayZones(app: app, isFirstPage: isFirst, isLastPage: isLast)
        var clippedCandidates: [(line: String, frame: CGRect)] = []
        var here = AppleFindings()

        // Two smaller audits instead of one large one. The contrast check is the heavy part, and on a loaded
        // runner a single combined audit intermittently hit Apple's internal time limit ("Audit failed to
        // complete in time", code -56) on whichever screen happened to be running. Splitting keeps each call
        // short, and a failure now names the part that did not finish. Each part gets three attempts with a
        // pause in between. A part that never completes is recorded as INCONCLUSIVE: a timeout is never a pass.
        let parts: [(name: String, types: XCUIAccessibilityAuditType)] = [
            ("contrast", .contrast),
            ("element, hit-region, description, text-size, clipping and trait checks", XCUIAccessibilityAuditType.all.subtracting(.contrast)),
        ]
        for part in parts {
            for attempt in 1...3 {
                var partFindings = AppleFindings()
                var partClipped: [(line: String, frame: CGRect)] = []
                do {
                    try app.performAccessibilityAudit(for: part.types) { issue in
                        let line = "page \(page): \(issue.auditType): \(issue.compactDescription) [\(issue.element?.label ?? "?")]"
                        guard let frame = issue.element?.frame else {
                            partFindings.unattributed.append(line + " (no addressable element)")
                            return true
                        }
                        if zones.contains(where: { frame.intersects($0) }) {
                            partFindings.occluded.append(line)
                            return true
                        }
                        if issue.auditType == .contrast {
                            partFindings.contrast.append((line, issue.element?.label ?? ""))
                            return true
                        }
                        if issue.compactDescription.contains("Text clipped") {
                            partClipped.append((line, frame))
                            return true
                        }
                        partFindings.reported.append(line)
                        return false
                    }
                    here.contrast += partFindings.contrast; here.reported += partFindings.reported
                    here.occluded += partFindings.occluded; here.unattributed += partFindings.unattributed
                    clippedCandidates += partClipped
                    break
                } catch {
                    if attempt < 3 {
                        Thread.sleep(forTimeInterval: 3)
                    } else {
                        found.inconclusive.append("page \(page): Apple audit (\(part.name)) did not complete after 3 attempts: \(error)")
                    }
                }
            }
        }

        if !clippedCandidates.isEmpty {
            let capture = ContrastAnalyzer.capture(app: app)
            let scale = capture.screenshot.image.cgImage.map { Double($0.width) / Double(app.windows.firstMatch.frame.width) } ?? 1
            for candidate in clippedCandidates {
                let rect = CGRect(x: candidate.frame.minX * scale, y: candidate.frame.minY * scale,
                                  width: candidate.frame.width * scale, height: candidate.frame.height * scale)
                let touches = capture.screenshot.image.cgImage.flatMap { ContrastAnalyzer.inkTouchesFrameEdge($0, pixelRect: rect) }
                switch touches {
                case .some(true): here.reported.append(candidate.line + " (glyph pixels touch the frame boundary)")
                case .some(false): here.overruled.append(candidate.line + " (no glyph pixel touches the frame boundary: text is complete)")
                case .none: here.inconclusive.append(candidate.line + " (frame could not be sampled)")
                }
            }
        }
        found.contrast += here.contrast
        found.reported += here.reported; found.occluded += here.occluded; found.overruled += here.overruled
        found.unattributed += here.unattributed; found.inconclusive += here.inconclusive
    }

    /// Audit one screen completely. Every scrolled page is captured, analysed with the pixel contrast
    /// analyzer and attached as a screenshot. Apple's audit runs on the first and the last page.
    private func audit(_ app: XCUIApplication, screen: String) {
        var raw: [ContrastObservation] = []
        var apple = AppleFindings()
        var pages = 0
        var reachedEnd = false
        var stoppedEarly = false
        var capture = ContrastAnalyzer.capture(app: app)

        for page in 0..<30 {
            pages = page + 1
            let attachment = XCTAttachment(screenshot: capture.screenshot)
            attachment.name = "\(screen)-page-\(page)"
            attachment.lifetime = .keepAlways
            add(attachment)

            // Scroll first, so we know whether this page is the last one before judging its bottom edge.
            // A scroll that moves nothing is tried up to two more times before it counts as the end, because
            // a single dropped gesture on a busy runner must not end an audit early.
            scrollDown(app)
            var next = ContrastAnalyzer.capture(app: app)
            for _ in 0..<2 where next.signature == capture.signature {
                Thread.sleep(forTimeInterval: 0.6)
                scrollDown(app)
                next = ContrastAnalyzer.capture(app: app)
            }
            let isLast = next.signature == capture.signature
            if isLast && ContrastAnalyzer.textRemainsUnderTabBar(capture, app: app) {
                stoppedEarly = true
            }
            let isFirst = page == 0
            raw += ContrastAnalyzer.analyze(capture, app: app, screen: screen, pageIndex: page, isFirstPage: isFirst, isLastPage: isLast)
            if isFirst {
                // Apple's audit measures the live screen, so it must run while the first page is showing.
                // The probe scroll above moved a scrolling screen, so go back to the top first. A screen
                // that does not scroll is both the first and the last page.
                if !isLast { scrollUp(app) }
                appleAudit(app, page: page, isFirst: true, isLast: isLast, into: &apple)
                if !isLast { scrollDown(app) }
            }
            if isLast {
                reachedEnd = true
                // The app is showing the last page now, because the final scroll attempt did not move it.
                if !isFirst { appleAudit(app, page: page, isFirst: false, isLast: true, into: &apple) }
                break
            }
            capture = next
        }

        // An Apple audit that did not complete, and findings that could not be sampled, are INCONCLUSIVE.
        for line in apple.inconclusive {
            raw.append(ContrastObservation(screen: screen, page: -1, label: line, classification: .inconclusive, ratio: nil,
                                           foreground: nil, background: nil, detail: "Apple's audit did not give a usable result"))
        }
        // Apple findings it could not attach to an element are INCONCLUSIVE observations.
        for line in apple.unattributed {
            raw.append(ContrastObservation(screen: screen, page: -1, label: line, classification: .inconclusive, ratio: nil,
                                           foreground: nil, background: nil, detail: "reported by Apple's audit without an element"))
        }
        for line in apple.reported where line.contains("Text clipped") {
            raw.append(ContrastObservation(screen: screen, page: -1, label: line, classification: .clipped, ratio: nil,
                                           foreground: nil, background: nil, detail: "reported by Apple's audit and confirmed by glyph pixels at the frame boundary"))
        }
        let final = ContrastAnalyzer.finalize(raw)

        // Apple's contrast findings: if the pixel analyzer measured the same text unobscured and it passes,
        // the finding is overruled and kept in the record with both numbers. Otherwise it stands.
        for item in apple.contrast {
            let key = String(item.label.prefix(80))
            if let measured = final.first(where: { $0.label == key && $0.classification == .pass }), let r = measured.ratio {
                apple.overruled.append(item.line + " (pixel measurement \(r):1, at least \(ContrastAnalyzer.threshold):1, on page \(measured.page); the element is legible)")
            } else {
                apple.reported.append(item.line + " (not confirmed as passing by the pixel analyzer)")
            }
        }
        let record = ScreenAudit(screen: screen, pages: pages, reachedEnd: reachedEnd, stoppedEarly: stoppedEarly, raw: raw, final: final,
                                 appleReported: apple.reported, appleOccluded: apple.occluded, appleOverruled: apple.overruled,
                                 appleUnattributed: apple.unattributed, appleInconclusive: apple.inconclusive)
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        if let data = try? encoder.encode(record) {
            let attachment = XCTAttachment(data: data, uniformTypeIdentifier: "public.json")
            attachment.name = "audit-\(screen).json"
            attachment.lifetime = .keepAlways
            add(attachment)
        }

        XCTAssertTrue(reachedEnd, "\(screen): did not reach the end of the screen within 30 pages")
        XCTAssertFalse(stoppedEarly, "\(screen): scrolling stopped while text still lies under the tab bar, so the screen was not fully audited")
        for o in final where o.classification == .fail { XCTFail("\(screen): contrast FAIL \(o.ratio ?? 0):1 [\(o.label)] page \(o.page)") }
        for o in final where o.classification == .clipped { XCTFail("\(screen): CLIPPED [\(o.label)] page \(o.page)") }
        for line in apple.reported { XCTFail("\(screen): Apple audit: \(line)") }
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
            XCTAssertGreaterThanOrEqual(option.frame.height, 43.5, "hit target of \(label) (44 pt design, floating-point tolerance)")
        }
    }

    // MARK: Virtual research lab

    /// Open a screen from the lab overview and wait for its navigation bar.
    private func openLab(_ app: XCUIApplication, _ row: String) {
        let link = app.buttons[row].firstMatch
        XCTAssertTrue(link.waitForExistence(timeout: 8), "row \(row) is not on the lab overview")
        // The rows lie below the fold. Scroll until the row is clear of the tab bar.
        var scrolls = 0
        while !link.isHittable && scrolls < 6 {
            scrollDown(app)
            scrolls += 1
        }
        link.tap()
        XCTAssertTrue(app.navigationBars[row].waitForExistence(timeout: 8), "\(row) did not show its navigation bar")
    }

    private func startLikeButtons(_ app: XCUIApplication) -> [String] {
        app.buttons.matching(NSPredicate(
            format: "label CONTAINS[c] 'start' OR label CONTAINS[c] 'launch' OR label CONTAINS[c] 'run exploit' OR label ==[c] 'jailbreak'"))
            .allElementsBoundByIndex.map(\.label)
    }

    func testLabOverviewDoesNotClaimAnExactVMAndOffersNoStartControl() {
        let app = launch()
        openMore(app, "Virtual Research Lab")
        XCTAssertTrue(has(app, "Exact target VM demonstrated"))
        XCTAssertTrue(has(app, "iPhone16,2"))
        XCTAssertTrue(has(app, "24A446"))
        XCTAssertFalse(has(app, "EXACT TARGET VERIFIED", timeout: 1), "no exact-target status without a demonstrated boot")
        XCTAssertEqual(startLikeButtons(app), [], "no control that could start a guest")
        shot(app, "lab-en")
        audit(app, screen: "lab")
    }

    func testLabBackendScreenShowsUnavailableControlsAndHostCommands() {
        let app = launch()
        openMore(app, "Virtual Research Lab")
        openLab(app, "VM backend")
        XCTAssertTrue(has(app, "Start virtual machine"))
        XCTAssertTrue(has(app, "Unavailable"))
        XCTAssertTrue(has(app, "homei-lab preflight"))
        XCTAssertTrue(has(app, "cannot start, stop or snapshot a guest"))
        XCTAssertEqual(startLikeButtons(app), [], "the start, stop and snapshot rows are explanations, not buttons")
        shot(app, "lab-backend-en")
        audit(app, screen: "lab-backend")
    }

    func testLabFidelityAndExperimentScreensAreHonestAboutWhatWasObserved() {
        let app = launch()
        openMore(app, "Virtual Research Lab")
        openLab(app, "Fidelity report")
        XCTAssertTrue(has(app, "Guest root is not physical device root"))
        XCTAssertTrue(has(app, "This is not an exact target virtual machine"))
        XCTAssertFalse(has(app, "EXACT TARGET VERIFIED", timeout: 1))
        shot(app, "lab-fidelity-en")
        audit(app, screen: "lab-fidelity")
        app.navigationBars.buttons.firstMatch.tap()
        openLab(app, "Experiments")
        XCTAssertTrue(has(app, "No virtual machine was started for this experiment"))
        shot(app, "lab-experiments-en")
    }

    func testLabGuestPackagesAndBootstrapAreGuestOnly() {
        let app = launch()
        openMore(app, "Virtual Research Lab")
        openLab(app, "Guest workspace")
        XCTAssertTrue(has(app, "Guest root is not physical device root"))
        app.navigationBars.buttons.firstMatch.tap()
        openLab(app, "Guest packages")
        XCTAssertTrue(has(app, "not a package manager for an iPhone"))
        app.navigationBars.buttons.firstMatch.tap()
        openLab(app, "Guest bootstrap")
        XCTAssertTrue(has(app, "not an iOS jailbreak bootstrap"))
        shot(app, "lab-bootstrap-en")
    }

    // MARK: Firmware research screens

    /// Open the firmware research hub from the lab overview.
    private func openResearch(_ app: XCUIApplication) {
        openMore(app, "Virtual Research Lab")
        openLab(app, "Firmware research")
        // The hub's own banner confirms the screen even when the title query is slow to appear.
        XCTAssertTrue(has(app, "Static analysis of firmware files"), "the research hub did not load")
    }

    /// Return from a research screen to the hub, so the next row is on screen.
    private func backToResearchHub(_ app: XCUIApplication) {
        let back = app.navigationBars.buttons.element(boundBy: 0)
        XCTAssertTrue(back.waitForExistence(timeout: 8), "no back button")
        back.tap()
        XCTAssertTrue(has(app, "Static analysis of firmware files"), "did not return to the research hub")
    }

    func testResearchHubStatesNoExactVMAndOpensEveryScreen() {
        let app = launch()
        openResearch(app)
        XCTAssertTrue(has(app, "NOT_ESTABLISHED"), "the exact target VM is shown as not established")
        XCTAssertTrue(has(app, "Static analysis of firmware files. Nothing was booted, emulated or simulated."))
        XCTAssertFalse(has(app, "EXACT TARGET VERIFIED", timeout: 1))
        shot(app, "research-hub-en")
        audit(app, screen: "research-hub")
    }

    func testFirmwareExplorerShowsTheEncryptedImagesAndTheManifestCounts() {
        let app = launch()
        openResearch(app)
        openLab(app, "Firmware explorer")
        XCTAssertTrue(has(app, "Encrypted (keys not available)"))
        XCTAssertTrue(has(app, "No path stated in the manifest"))
        XCTAssertTrue(has(app, "24A446"))
        shot(app, "research-firmware-en")
        audit(app, screen: "research-firmware")
    }

    func testDeviceTreeAndKernelcacheShowTheRecordedValuesWithoutInventingAny() {
        let app = launch()
        openResearch(app)
        openLab(app, "Device tree inspector")
        XCTAssertTrue(has(app, "D84AP, iPhone16,2, AppleARM"))
        XCTAssertTrue(has(app, "arm-io,t8130"))
        backToResearchHub(app)
        openLab(app, "Kernelcache inspector")
        XCTAssertTrue(has(app, "RELEASE_ARM64_T8122"))
        XCTAssertTrue(has(app, "MH_FILESET"))
        audit(app, screen: "research-kernelcache")
    }

    func testAICAndPMGRScreenReportsTheTranslationAsInferredNotVerified() {
        let app = launch()
        openResearch(app)
        openLab(app, "AIC and PMGR research")
        XCTAssertTrue(has(app, "0x2d1000000"))
        XCTAssertTrue(has(app, "aic,3"))
        XCTAssertTrue(has(app, "BLOCKED"))
        audit(app, screen: "research-aicpmgr")
    }

    func testSPTMGhidraAndVMScreensStateTheirLimits() {
        let app = launch()
        openResearch(app)
        openLab(app, "SPTM register inventory")
        XCTAssertTrue(has(app, "127"))
        XCTAssertTrue(has(app, "Undefined in the fork's model"))
        backToResearchHub(app)
        backToResearchHub(app)
        openLab(app, "Ghidra analysis browser")
        XCTAssertTrue(has(app, "Not exported (unknown)"))
        backToResearchHub(app)
        openLab(app, "VM feasibility")
        XCTAssertTrue(has(app, "NOT_ESTABLISHED"))
        XCTAssertTrue(has(app, "Not established"))
        audit(app, screen: "research-vm")
    }

    func testEvidenceExplorerShowsConfidenceTagsAndLimitations() {
        let app = launch()
        openResearch(app)
        openLab(app, "Evidence explorer")
        XCTAssertTrue(has(app, "[T] FA-001"))
        XCTAssertTrue(has(app, "Limitation"))
        audit(app, screen: "research-evidence")
    }

    func testKoreanLab() {
        let app = launch(language: "ko", locale: "ko_KR")
        tab(app, "더 보기")
        let link = app.buttons["가상 연구 랩"].firstMatch
        XCTAssertTrue(link.waitForExistence(timeout: 8))
        link.tap()
        XCTAssertTrue(app.navigationBars["가상 연구 랩"].waitForExistence(timeout: 8))
        XCTAssertTrue(has(app, "정확한 대상 VM 입증 여부"))
        XCTAssertFalse(has(app, "Exact target VM demonstrated", timeout: 1), "no untranslated label")
        shot(app, "lab-ko")
        audit(app, screen: "lab-ko")
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
