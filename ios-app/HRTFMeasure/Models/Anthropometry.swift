import Foundation

/// Mirrors the Rust `Anthropometry` struct in synthesizer.rs.
/// All measurements are in centimeters.
struct Anthropometry: Codable, Equatable {
    var headWidth: Float
    var headDepth: Float
    var earParamsLeft: EarParameters
    var earParamsRight: EarParameters

    static let defaultValue = Anthropometry(
        headWidth: 14.5,
        headDepth: 19.5,
        earParamsLeft: .defaultValue,
        earParamsRight: .defaultValue
    )

    /// Validation ranges matching the Rust GUI (gui/mod.rs:333-392)
    static let headWidthRange: ClosedRange<Float> = 10.0...25.0
    static let headDepthRange: ClosedRange<Float> = 15.0...30.0

    var isValid: Bool {
        Self.headWidthRange.contains(headWidth)
            && Self.headDepthRange.contains(headDepth)
            && earParamsLeft.isValid
            && earParamsRight.isValid
    }
}
