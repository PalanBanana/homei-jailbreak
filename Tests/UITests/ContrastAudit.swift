import CoreGraphics
import Foundation
import UIKit
import XCTest

/// Result categories for one text element. None of them is silently converted into another.
///
/// - pass: unobscured, fully visible, rendered contrast at or above the threshold.
/// - fail: unobscured, fully visible, rendered contrast below the threshold.
/// - occluded: the element was only ever seen inside a system bar or the scroll-edge blur.
/// - clipped: the element is cut off by the screen edge, or glyph pixels touch its frame boundary.
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

/// One page of a screen: the screenshot and the text elements, both taken at the same moment.
struct PageCapture {
    let screenshot: XCUIScreenshot
    let texts: [(label: String, frame: CGRect)]

    /// Identity of the scroll position. It changes when content moves and is identical when it does
    /// not, so it detects the end of a screen without being fooled by scroll indicators or the clock.
    var signature: String {
        texts.map { "\($0.label)@\(Int($0.frame.minY.rounded()))" }.sorted().joined(separator: "|")
    }
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
/// - The crop is inset from the frame so rounded corners of a container cannot be mistaken for text.
///   A crop that still mixes several background colours is measured against its dominant one.
enum ContrastAnalyzer {
    static let threshold = 4.5

    /// Height, in points, of the soft blur the system draws beyond the bar frames while content
    /// scrolls under them. Text inside this zone is rendered faded, so it is OCCLUDED, not measured.
    static let edgeMargin: CGFloat = 64

    /// The regions where the system draws bars or scroll-edge effects over the content.
    ///
    /// The scroll-edge blur only exists where content can scroll under a bar. On the first page there
    /// is nothing above the content, and on the last page nothing below it, so only the bar frames
    /// themselves count there. A screen that does not scroll is both first and last.
    static func overlayZones(app: XCUIApplication, isFirstPage: Bool, isLastPage: Bool) -> [CGRect] {
        let window = app.windows.firstMatch.frame
        var zones: [CGRect] = []
        let nav = app.navigationBars.firstMatch
        if nav.exists {
            zones.append(CGRect(x: 0, y: 0, width: window.width, height: nav.frame.maxY + (isFirstPage ? 0 : edgeMargin)))
        }
        let tab = app.tabBars.firstMatch
        if tab.exists {
            let top = tab.frame.minY - (isLastPage ? 0 : edgeMargin)
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

    // MARK: Pixel access

    /// RGBA bytes for a crop of the image, or nil if the crop is unusable.
    static func pixels(_ image: CGImage, crop: CGRect) -> (data: [UInt8], width: Int, height: Int)? {
        let bounds = CGRect(x: 0, y: 0, width: image.width, height: image.height)
        let rect = crop.integral.intersection(bounds)
        guard rect.width >= 3, rect.height >= 3, let cropped = image.cropping(to: rect) else { return nil }
        let w = cropped.width, h = cropped.height
        var data = [UInt8](repeating: 0, count: w * h * 4)
        let space = CGColorSpaceCreateDeviceRGB()
        let drawn = data.withUnsafeMutableBytes { buffer -> Bool in
            guard let ctx = CGContext(data: buffer.baseAddress, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4,
                                      space: space, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return false }
            ctx.draw(cropped, in: CGRect(x: 0, y: 0, width: w, height: h))
            return true
        }
        return drawn ? (data, w, h) : nil
    }

    struct Bucket { var count = 0; var r = 0.0; var g = 0.0; var b = 0.0 }

    static func buckets(_ data: [UInt8]) -> [Int: Bucket] {
        var out: [Int: Bucket] = [:]
        for i in stride(from: 0, to: data.count, by: 4) {
            let r = Int(data[i]), g = Int(data[i + 1]), b = Int(data[i + 2])
            let key = (r >> 4) << 8 | (g >> 4) << 4 | (b >> 4)
            var bucket = out[key] ?? Bucket()
            bucket.count += 1; bucket.r += Double(r); bucket.g += Double(g); bucket.b += Double(b)
            out[key] = bucket
        }
        return out
    }

    static func colour(_ b: Bucket) -> (Double, Double, Double) {
        let n = Double(b.count) * 255
        return (b.r / n, b.g / n, b.b / n)
    }

    // MARK: Measurement

    /// Inset, in points, applied to every side of a frame before measuring. A rounded container clears its
    /// corners at about 15% of its height, so the inset grows with the element, between 8 and 24 points.
    static func inset(forHeight height: CGFloat) -> CGFloat { min(max(8, height * 0.18), 24) }

    struct Measurement {
        let ratio: Double
        let foreground: (Double, Double, Double)
        let background: (Double, Double, Double)
    }

    /// Returns nil when there are too few glyph pixels to say anything. `insetPixels` shrinks the crop
    /// on every side before measuring.
    static func measure(_ image: CGImage, pixelRect: CGRect, insetPixels: CGFloat) -> Measurement? {
        let inset = min(insetPixels, pixelRect.width / 4, pixelRect.height / 4)
        guard let crop = pixels(image, crop: pixelRect.insetBy(dx: inset, dy: inset)) else { return nil }
        let all = buckets(crop.data)
        guard let backgroundKey = all.max(by: { $0.value.count < $1.value.count })?.key,
              let backgroundBucket = all[backgroundKey] else { return nil }
        let background = colour(backgroundBucket)

        // Glyph colours: buckets that visibly differ from the background, ignoring the faint
        // anti-aliasing buckets that each hold only a small share of the pixels.
        let candidates = all.filter { $0.key != backgroundKey && ratio(colour($0.value), background) > 1.15 }
        guard let strongest = candidates.values.map(\.count).max(), strongest >= 6 else { return nil }
        let significant = candidates.values.filter { Double($0.count) >= 0.25 * Double(strongest) }
        // The weakest significant glyph colour decides the result, so a low-contrast colour cannot
        // hide behind a high-contrast one in the same element.
        guard let worst = significant.min(by: { ratio(colour($0), background) < ratio(colour($1), background) }) else { return nil }
        let fg = colour(worst)
        return Measurement(ratio: ratio(fg, background), foreground: fg, background: background)
    }

    /// True when strongly contrasting pixels sit on the outermost rows or columns of the element's
    /// frame. Text that fits leaves a margin inside its frame, so ink on the boundary means the glyphs
    /// are cut off there. Nil when the crop cannot be read.
    static func inkTouchesFrameEdge(_ image: CGImage, pixelRect: CGRect) -> Bool? {
        guard let crop = pixels(image, crop: pixelRect) else { return nil }
        let all = buckets(crop.data)
        guard let backgroundKey = all.max(by: { $0.value.count < $1.value.count })?.key,
              let backgroundBucket = all[backgroundKey] else { return nil }
        let background = colour(backgroundBucket)
        let w = crop.width, h = crop.height
        var inkOnEdge = 0
        func isInk(_ x: Int, _ y: Int) -> Bool {
            let i = (y * w + x) * 4
            let c = (Double(crop.data[i]) / 255, Double(crop.data[i + 1]) / 255, Double(crop.data[i + 2]) / 255)
            return ratio(c, background) > 3.0
        }
        for x in 0..<w {
            if isInk(x, 0) { inkOnEdge += 1 }
            if isInk(x, h - 1) { inkOnEdge += 1 }
        }
        for y in 0..<h {
            if isInk(0, y) { inkOnEdge += 1 }
            if isInk(w - 1, y) { inkOnEdge += 1 }
        }
        return inkOnEdge >= 4
    }

    // MARK: Page analysis

    /// Take the screenshot and the text elements together, in one pass over the accessibility tree.
    /// Reading label and frame through separate element queries costs an inter-process call each and
    /// is far slower than one snapshot.
    static func capture(app: XCUIApplication) -> PageCapture {
        var texts: [(label: String, frame: CGRect)] = []
        func collect(_ node: XCUIElementSnapshot) {
            if node.elementType == .staticText { texts.append((node.label, node.frame)) }
            for child in node.children { collect(child) }
        }
        let shot = app.screenshot()
        if let snapshot = try? app.snapshot() { collect(snapshot) }
        return PageCapture(screenshot: shot, texts: texts)
    }

    /// Classify every text element visible on a captured page.
    static func analyze(_ page: PageCapture, app: XCUIApplication, screen: String, pageIndex: Int,
                        isFirstPage: Bool, isLastPage: Bool) -> [ContrastObservation] {
        guard let image = page.screenshot.image.cgImage, !page.texts.isEmpty else {
            return [ContrastObservation(screen: screen, page: pageIndex, label: "<page>", classification: .inconclusive, ratio: nil,
                                        foreground: nil, background: nil, detail: "no screenshot bitmap or no text elements")]
        }
        let window = app.windows.firstMatch.frame
        let scale = Double(image.width) / Double(window.width)
        let tabBar = app.tabBars.firstMatch.exists ? app.tabBars.firstMatch.frame : .zero
        let navBar = app.navigationBars.firstMatch.exists ? app.navigationBars.firstMatch.frame : .zero
        let zones = overlayZones(app: app, isFirstPage: isFirstPage, isLastPage: isLastPage)

        var seen = Set<String>()
        var out: [ContrastObservation] = []
        for (label, frame) in page.texts {
            guard !label.isEmpty, frame.width >= 4, frame.height >= 4, frame.maxY > 0, frame.minY < window.height else { continue }
            if frame.height > window.height * 0.7 { continue }                                  // a container, not a text run
            if tabBar != .zero, tabBar.insetBy(dx: -2, dy: -2).contains(frame) { continue }    // system tab bar text
            if navBar != .zero, navBar.insetBy(dx: -2, dy: -2).contains(frame) { continue }    // system title and back button
            let key = "\(label)|\(Int(frame.minX))|\(Int(frame.minY))"
            guard seen.insert(key).inserted else { continue }

            func obs(_ c: ContrastClass, _ detail: String, _ m: Measurement? = nil) -> ContrastObservation {
                ContrastObservation(screen: screen, page: pageIndex, label: String(label.prefix(80)), classification: c,
                                    ratio: m.map { (($0.ratio) * 100).rounded() / 100 },
                                    foreground: m.map { hex($0.foreground) }, background: m.map { hex($0.background) }, detail: detail)
            }

            var measured = frame
            if zones.contains(where: { frame.intersects($0) }) {
                // Measure the part of the element that lies between the zones. If too little of it does,
                // for example a short line inside the blur, it stays OCCLUDED and is never counted as a pass.
                let top = zones.filter { $0.minY <= 0 }.map(\.maxY).max() ?? 0
                let bottom = zones.filter { $0.maxY >= window.height }.map(\.minY).min() ?? window.height
                let band = CGRect(x: 0, y: top, width: window.width, height: max(0, bottom - top))
                let visible = frame.intersection(band)
                guard !visible.isNull, visible.height >= 18 else {
                    out.append(obs(.occluded, "inside a system bar or its scroll-edge zone"))
                    continue
                }
                measured = visible
            }
            if frame.minX < -0.5 || frame.maxX > window.width + 0.5 || frame.minY < -0.5 || frame.maxY > window.height + 0.5 {
                out.append(obs(.clipped, "frame is cut by the screen edge"))
                continue
            }
            let pixelRect = CGRect(x: measured.minX * scale, y: measured.minY * scale, width: measured.width * scale, height: measured.height * scale)
            let partial = measured.height < frame.height - 0.5
            guard let m = measure(image, pixelRect: pixelRect, insetPixels: inset(forHeight: measured.height) * scale) else {
                out.append(obs(.inconclusive, "too few glyph pixels to measure"))
                continue
            }
            out.append(obs(m.ratio >= threshold ? .pass : .fail,
                           "measured \(String(format: "%.2f", m.ratio)):1 against \(threshold):1" + (partial ? " on the part outside the system zones" : ""), m))
        }
        return out
    }

    /// Merge per-page observations into one final class per element. Evidence that the element
    /// was measured while unobscured wins over occlusion events on other pages, and a FAIL on any
    /// page wins over a PASS on another. OCCLUDED and CLIPPED survive only for elements that were
    /// never measured. Elements with identical text are merged.
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
