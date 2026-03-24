import Foundation

/// Six ear parameters per ear, following the CIPIC convention.
/// Indices d4 and d6 are excluded (not used by the neural network).
/// All values in centimeters.
struct EarParameters: Codable, Equatable {
    /// Cavum concha height (typical 1.4–2.2 cm)
    var d1: Float
    /// Cymba concha height (typical 0.5–1.2 cm)
    var d2: Float
    /// Cavum concha width (typical 1.3–2.1 cm)
    var d3: Float
    /// Pinna height (typical 5.5–7.5 cm)
    var d5: Float
    /// Pinna width (typical 0.3–0.9 cm)
    var d7: Float
    /// Ear canal entrance to rear of pinna (typical 0.9–1.7 cm)
    var d8: Float

    /// Validation range for all ear parameters (matching Rust GUI)
    static let paramRange: ClosedRange<Float> = 0.1...8.0

    static let defaultValue = EarParameters(
        d1: 0.9, d2: 0.5, d3: 0.8,
        d5: 3.0, d7: 1.5, d8: 1.6
    )

    /// Returns the 6 values as an array in neural network input order: [d1, d2, d3, d5, d7, d8]
    var asArray: [Float] {
        [d1, d2, d3, d5, d7, d8]
    }

    var isValid: Bool {
        asArray.allSatisfy { Self.paramRange.contains($0) }
    }

    /// The three macro parameters that can be measured directly via LiDAR/photo
    var macroParams: (d5: Float, d7: Float, d8: Float) {
        (d5, d7, d8)
    }

    /// The three concha parameters that require ML regression
    var conchaParams: (d1: Float, d2: Float, d3: Float) {
        (d1, d2, d3)
    }

    /// Update concha parameters from ML regression output
    mutating func updateConcha(d1: Float, d2: Float, d3: Float) {
        self.d1 = d1
        self.d2 = d2
        self.d3 = d3
    }
}
