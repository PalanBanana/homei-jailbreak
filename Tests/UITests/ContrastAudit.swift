import CoreGraphics
import Foundation
import UIKit
import XCTest

/// Result categories for one text element. None of them is silently converted into another.
///
/// - pass: unobscured, fully visible, rendered contrast at or above the threshold.
/// - fail: unobscured, fully visible, rendered contrast below the threshold.
/// - occluded: the element overlaps the system tab bar or navigation bar on every page it appeared.
/// - clipped: the element is cut off by the screen edge on every page, or Apple's audit reported its text clipped.
/// - inconclusive: the element could not be measured reliably (too few glyph pixels, no usable crop).
enum ContrastClass: String, Codable {
    case pass = "PASS", fail = "FAIL", occluded = "OCCLUDED", clipped = "CLIPPED", inconclusive = "INCONCLUSIVE"
}

struct ContrastObservation: Codable {
    let screen: String
    let page: Int
    let label: String
    let classification: ContrastClass
    let ratio: Double?
    let foreground: String?
    let background: String?
    let detail: String
}

/// Measures the contrast the user actually sees: it crops the rendered screenshot to each text
/// element's frame, finds the effective background (the dominant colour of the crop, which already
/// includes tints and translucency as composited on screen) and the glyph colours, and applies the
/// WCAG 2.x relative-luminance formula.
///
/// Limits, stated so nobody mistakes this for more than it is:
/// - Colours are estimated from pixels. Anti-aliasing and very small text lower accuracy.
/// - The 4.5:1 threshold is applied to every element. The 3:1 allowance for large text is not used,
///   which is stricter than WCAG.
/// - A crop that mixes several background colours is measured against its dominant one.
enum ContrastAnalyzer {
    static let threshold = 4.5

    /// Height, in points, of the soft blur the system draws beyond the bar frames while content
    /// scrolls under them. Text inside this zone is rendered faded, so it is OCCLUDED, not measured.
    static let edgeMargin: CGFloat = 56

    /// The regions where the system draws bars or scroll-edge effects over the content.
    static func overlayZones(app: XCUIApplication) -> [CGRect] {
        let window = app.windows.firstMatch.frame
        var zones: [CGRect] = []
        let nav = app.navigationBars.firstMatch
        if nav.exists { zones.append(CGRect(x: 0, y: 0, width: window.width, height: nav.frame.maxY + edgeMargin)) }
        let tab = app.tabBars.firstMatch
        if tab.exists {
            let top = tab.frame.minY - edgeMargin
            zones.append(CGRect(x: 0, y: top, width: window.width, height: window.height - top))
        }
        return zones
    }

    // MARK: WCAG maths

    static func channel(_ v: Double) -> Double { v <= 0.03928 ? v / 12.92 : pow((v + 0.055) / 1.055, 2.4) }

    static func luminance(_ c: (Double, Double, Double)) -> Double {
        0.2126 * channel(c.0) + 0.7152 * channel(c.1) + 0.0722 * channel(c.2)
    }

    static func ratio(_ a: (Double, Double, Double), _ b: (Double, Double, Double)) -> Double {
        let la = luminance(a), lb = luminance(b)
        return (max(la, lb) + 0.05) / (min(la, lb) + 0.05)
    }

    static func hex(_ c: (Double, Double, Double)) -> String {
        String(format: "#%02X%02X%02X", Int((c.0 * 255).rounded()), Int((c.1 * 255).rounded()), Int((c.2 * 255).rounded()))
    }

    // MARK: Pixel measurement

    struct Measurement {
        let ratio: Double
        let foreground: (Double, Double, Double)
        let background: (Double, Double, Double)
    }

    /// Returns nil when there are too few glyph pixels to say anything.
    static func measure(_ image: CGImage, pixelRect: CGRect) -> Measurement? {
        let bounds = CGRect(x: 0, y: 0, width: image.width, height: image.height)
        let crop = pixelRect.integral.intersection(bounds)
        guard crop.width >= 4, crop.height >= 4, let cropped = image.cropping(to: crop) else { return nil }
        let w = cropped.width, h = cropped.height
        var data = [UInt8](repeating: 0, count: w * h * 4)
        let space = CGColorSpaceCreateDeviceRGB()
        let drawn = data.withUnsafeMutableBytes { buffer -> Bool in
            guard let ctx = CGContext(data: buffer.baseAddress, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4,
                                      space: space, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return false }
            ctx.draw(cropped, in: CGRect(x: 0, y: 0, width: w, height: h))
            return true
        }
        guard drawn else { return nil }

        // Bucket pixels at 4 bits per channel and keep the exact running colour of each bucket.
        struct Bucket { var count = 0; var r = 0.0; var g = 0.0; var b = 0.0 }
        var buckets: [Int: Bucket] = [:]
        for i in stride(from: 0, to: data.count, by: 4) {
            let r = Int(data[i]), g = Int(data[i + 1]), b = Int(data[i + 2])
            let key = (r >> 4) << 8 | (g >> 4) << 4 | (b >> 4)
            var bucket = buckets[key] ?? Bucket()
            bucket.count += 1; bucket.r += Double(r); bucket.g += Double(g); bucket.b += Double(b)
            buckets[key] = bucket
        }
        func colour(_ b: Bucket) -> (Double, Double, Double) {
            let n = Double(b.count) * 255
            return (b.r / n, b.g / n, b.b / n)
        }
        guard let backgroundKey = buckets.max(by: { $0.value.count < $1.value.count })?.key,
              let backgroundBucket = buckets[backgroundKey] else { return nil }
        let background = colour(backgroundBucket)

        // Glyph colours: buckets that visibly differ from the background, ignoring the faint
        // anti-aliasing buckets that each hold only a small share of the pixels.
        let candidates = buckets.filter { $0.key != backgroundKey && ratio(colour($0.value), background) > 1.15 }
        guard let strongest = candidates.values.map(\.count).max(), strongest >= 6 else { return nil }
        let significant = candidates.values.filter { Double($0.count) >= 0.25 * Double(strongest) }
        // The weakest significant glyph colour decides the result, so a low-contrast colour cannot
        // hide behind a high-contrast one in the same element.
        guard let worst = significant.min(by: { ratio(colour($0), background) < ratio(colour($1), background) }) else { return nil }
        let fg = colour(worst)
        return Measurement(ratio: ratio(fg, background), foreground: fg, background: background)
    }

    // MARK: Page analysis

    /// Classify every text element visible on the current page.
    static func analyzePage(app: XCUIApplication, screen: String, page: Int) -> [ContrastObservation] {
        let screenshot = app.screenshot()
        guard let image = screenshot.image.cgImage else {
            return [ContrastObservation(screen: screen, page: page, label: "<page>", classification: .inconclusive, ratio: nil,
                                        foreground: nil, background: nil, detail: "screenshot has no bitmap")]
        }
        let window = app.windows.firstMatch.frame
        let scale = Double(image.width) / Double(window.width)
        let tabBar = app.tabBars.firstMatch.exists ? app.tabBars.firstMatch.frame : .zero
        let navBar = app.navigationBars.firstMatch.exists ? app.navigationBars.firstMatch.frame : .zero

        let zones = overlayZones(app: app)
        var seen = Set<String>()
        var out: [ContrastObservation] = []
        let elements = app.descendants(matching: .staticText).allElementsBoundByIndex
        for element in elements {
            let label = element.label
            let frame = element.frame
            guard !label.isEmpty, frame.width >= 4, frame.height >= 4, frame.maxY > 0, frame.minY < window.height else { continue }
            if frame.height > window.height * 0.7 { continue }                      // a container, not a text run
            if tabBar != .zero, tabBar.insetBy(dx: -2, dy: -2).contains(frame) { continue }   // system tab bar text
            if navBar != .zero, navBar.insetBy(dx: -2, dy: -2).contains(frame) { continue }   // system title and back button
            let key = "\(label)|\(Int(frame.minX))|\(Int(frame.minY))"
            guard seen.insert(key).inserted else { continue }

            func obs(_ c: ContrastClass, _ detail: String, _ m: Measurement? = nil) -> ContrastObservation {
                ContrastObservation(screen: screen, page: page, label: String(label.prefix(80)), classification: c,
                                    ratio: m.map { (($0.ratio) * 100).rounded() / 100 },
                                    foreground: m.map { hex($0.foreground) }, background: m.map { hex($0.background) }, detail: detail)
            }

            if zones.contains(where: { frame.intersects($0) }) {
                out.append(obs(.occluded, "inside a system bar or its scroll-edge zone"))
                continue
            }
            if frame.minX < -0.5 || frame.maxX > window.width + 0.5 || frame.minY < -0.5 || frame.maxY > window.height + 0.5 {
                out.append(obs(.clipped, "frame is cut by the screen edge"))
                continue
            }
            let pixelRect = CGRect(x: frame.minX * scale, y: frame.minY * scale, width: frame.width * scale, height: frame.height * scale)
            guard let m = measure(image, pixelRect: pixelRect) else {
                out.append(obs(.inconclusive, "too few glyph pixels to measure"))
                continue
            }
            out.append(obs(m.ratio >= threshold ? .pass : .fail, "measured \(String(format: "%.2f", m.ratio)):1 against \(threshold):1", m))
        }
        return out
    }

    /// Merge per-page observations into one final class per element. Evidence that the element
    /// was measured while unobscured wins over occlusion events on other pages, and a FAIL on any
    /// page wins over a PASS on another. OCCLUDED and CLIPPED survive only for elements that were
    /// never measured.
    static func finalize(_ observations: [ContrastObservation]) -> [ContrastObservation] {
        var byLabel: [String: [ContrastObservation]] = [:]
        for o in observations { byLabel[o.label, default: []].append(o) }
        var final: [ContrastObservation] = []
        for (_, group) in byLabel {
            if let fail = group.filter({ $0.classification == .fail }).min(by: { ($0.ratio ?? 0) < ($1.ratio ?? 0) }) {
                final.append(fail)
            } else if let pass = group.filter({ $0.classification == .pass }).min(by: { ($0.ratio ?? 0) < ($1.ratio ?? 0) }) {
                final.append(pass)
            } else if let inconclusive = group.first(where: { $0.classification == .inconclusive }) {
                final.append(inconclusive)
            } else if let clipped = group.first(where: { $0.classification == .clipped }) {
                final.append(clipped)
            } else if let occluded = group.first {
                final.append(occluded)
            }
        }
        return final.sorted { ($0.page, $0.label) < ($1.page, $1.label) }
    }
}
