import Foundation
import CoreML

/// Wraps CoreML model inference for ear measurement estimation.
enum MLService {

    /// Predict concha dimensions (d1, d2, d3) from head + macro ear measurements.
    /// Uses the ConchaRegression CoreML model.
    static func predictConchaParams(
        headWidth: Float,
        headDepth: Float,
        d5: Float,
        d7: Float,
        d8: Float
    ) -> (d1: Float, d2: Float, d3: Float, confidence: Float)? {
        // Try to load the CoreML model
        guard let model = loadConchaRegressionModel() else {
            // Fallback: use simple linear regression coefficients
            // derived from CIPIC/ARI database correlations
            return fallbackConchaEstimate(
                headWidth: headWidth, headDepth: headDepth,
                d5: d5, d7: d7, d8: d8
            )
        }

        do {
            let input = try MLMultiArray(shape: [5], dataType: .float32)
            input[0] = NSNumber(value: headWidth)
            input[1] = NSNumber(value: headDepth)
            input[2] = NSNumber(value: d5)
            input[3] = NSNumber(value: d7)
            input[4] = NSNumber(value: d8)

            let prediction = try model.prediction(from: MLDictionaryFeatureProvider(
                dictionary: ["input": MLFeatureValue(multiArray: input)]
            ))

            guard let output = prediction.featureValue(for: "output")?.multiArrayValue else {
                return nil
            }

            return (
                d1: output[0].floatValue,
                d2: output[1].floatValue,
                d3: output[2].floatValue,
                confidence: 0.6  // ML regression confidence
            )
        } catch {
            print("CoreML inference failed: \(error)")
            return fallbackConchaEstimate(
                headWidth: headWidth, headDepth: headDepth,
                d5: d5, d7: d7, d8: d8
            )
        }
    }

    /// Detect ear landmarks from a photo using the EarLandmarks CoreML model.
    /// Returns landmark positions in normalized image coordinates.
    static func detectEarLandmarks(imageData: Data) -> [String: CGPoint]? {
        // Placeholder for Phase 3 - ear landmark detection model
        // Will be implemented when the EarLandmarks.mlmodel is trained
        return nil
    }

    // MARK: - Private

    private static func loadConchaRegressionModel() -> MLModel? {
        guard let url = Bundle.main.url(forResource: "ConchaRegression", withExtension: "mlmodelc") else {
            return nil
        }
        return try? MLModel(contentsOf: url)
    }

    /// Fallback concha estimation using empirical proportions from CIPIC database.
    /// These coefficients approximate the relationship between macro and concha params.
    private static func fallbackConchaEstimate(
        headWidth: Float, headDepth: Float,
        d5: Float, d7: Float, d8: Float
    ) -> (d1: Float, d2: Float, d3: Float, confidence: Float) {
        // Simple proportional estimates based on CIPIC database averages:
        // d1 (cavum concha height) ≈ 0.28 * d5
        // d2 (cymba concha height) ≈ 0.13 * d5
        // d3 (cavum concha width)  ≈ 0.23 * d5 + 0.15 * d8
        let d1 = 0.28 * d5
        let d2 = 0.13 * d5
        let d3 = 0.23 * d5 + 0.15 * d8

        return (
            d1: max(EarParameters.paramRange.lowerBound, min(d1, EarParameters.paramRange.upperBound)),
            d2: max(EarParameters.paramRange.lowerBound, min(d2, EarParameters.paramRange.upperBound)),
            d3: max(EarParameters.paramRange.lowerBound, min(d3, EarParameters.paramRange.upperBound)),
            confidence: 0.4  // lower confidence for proportional fallback
        )
    }
}
