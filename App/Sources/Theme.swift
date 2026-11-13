import SwiftUI

/// Original HOMEI design tokens: deep navy surfaces with a cyan-blue accent.
enum Theme {
    static let background = Color(red: 0.04, green: 0.07, blue: 0.13)
    static let surface = Color(red: 0.07, green: 0.11, blue: 0.20)
    static let surfaceRaised = Color(red: 0.10, green: 0.16, blue: 0.28)
    static let accent = Color(red: 0.25, green: 0.62, blue: 1.0)
    static let textPrimary = Color(red: 0.93, green: 0.96, blue: 1.0)
    static let textSecondary = Color(red: 0.62, green: 0.70, blue: 0.82)
    static let positive = Color(red: 0.27, green: 0.78, blue: 0.55)
    static let caution = Color(red: 0.98, green: 0.72, blue: 0.25)
    static let negative = Color(red: 0.95, green: 0.40, blue: 0.40)
    static let neutral = Color(red: 0.55, green: 0.62, blue: 0.74)
    static let radius: CGFloat = 14
    static let spacing: CGFloat = 16
}
