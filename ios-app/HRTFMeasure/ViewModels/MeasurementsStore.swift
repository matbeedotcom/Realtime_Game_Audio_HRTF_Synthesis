import Foundation
import Combine

/// Central observable store for all anthropometric measurements.
/// Shared across views via @EnvironmentObject.
final class MeasurementsStore: ObservableObject {
    @Published var anthropometry = Anthropometry.defaultValue
    @Published var confidence = MeasurementConfidence.unknown
    @Published var scanState = ScanState.notStarted

    enum ScanState: Equatable {
        case notStarted
        case scanningHead
        case scanningLeftEar
        case scanningRightEar
        case reviewingResults
        case exported
    }

    // MARK: - Head measurements

    func updateHeadWidth(_ width: Float, confidence: Float) {
        anthropometry.headWidth = width
        self.confidence.headWidth = confidence
    }

    func updateHeadDepth(_ depth: Float, confidence: Float) {
        anthropometry.headDepth = depth
        self.confidence.headDepth = confidence
    }

    // MARK: - Ear measurements

    func updateLeftEarMacro(d5: Float, d7: Float, d8: Float, confidence: Float) {
        anthropometry.earParamsLeft.d5 = d5
        anthropometry.earParamsLeft.d7 = d7
        anthropometry.earParamsLeft.d8 = d8
        self.confidence.earLeft.d5 = confidence
        self.confidence.earLeft.d7 = confidence
        self.confidence.earLeft.d8 = confidence
    }

    func updateRightEarMacro(d5: Float, d7: Float, d8: Float, confidence: Float) {
        anthropometry.earParamsRight.d5 = d5
        anthropometry.earParamsRight.d7 = d7
        anthropometry.earParamsRight.d8 = d8
        self.confidence.earRight.d5 = confidence
        self.confidence.earRight.d7 = confidence
        self.confidence.earRight.d8 = confidence
    }

    func updateLeftEarConcha(d1: Float, d2: Float, d3: Float, confidence: Float) {
        anthropometry.earParamsLeft.updateConcha(d1: d1, d2: d2, d3: d3)
        self.confidence.earLeft.d1 = confidence
        self.confidence.earLeft.d2 = confidence
        self.confidence.earLeft.d3 = confidence
    }

    func updateRightEarConcha(d1: Float, d2: Float, d3: Float, confidence: Float) {
        anthropometry.earParamsRight.updateConcha(d1: d1, d2: d2, d3: d3)
        self.confidence.earRight.d1 = confidence
        self.confidence.earRight.d2 = confidence
        self.confidence.earRight.d3 = confidence
    }

    // MARK: - Export

    func exportJSON() throws -> Data {
        let export = MeasurementExport(from: anthropometry, confidence: confidence)
        return try export.toJSON()
    }

    func reset() {
        anthropometry = .defaultValue
        confidence = .unknown
        scanState = .notStarted
    }
}
