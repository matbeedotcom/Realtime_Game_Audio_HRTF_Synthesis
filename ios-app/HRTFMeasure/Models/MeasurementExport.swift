import Foundation

/// JSON export format for transferring measurements to the desktop HRTF synthesizer.
struct MeasurementExport: Codable {
    let version: Int = 1
    let headWidth: Float
    let headDepth: Float
    let earLeft: EarExport
    let earRight: EarExport
    let confidence: ConfidenceExport?

    struct EarExport: Codable {
        let d1: Float
        let d2: Float
        let d3: Float
        let d5: Float
        let d7: Float
        let d8: Float
    }

    struct ConfidenceExport: Codable {
        let headWidth: Float
        let headDepth: Float
        let earLeft: EarExport
        let earRight: EarExport
    }

    enum CodingKeys: String, CodingKey {
        case version
        case headWidth = "head_width"
        case headDepth = "head_depth"
        case earLeft = "ear_left"
        case earRight = "ear_right"
        case confidence
    }

    init(from anthropometry: Anthropometry, confidence: MeasurementConfidence) {
        self.headWidth = anthropometry.headWidth
        self.headDepth = anthropometry.headDepth
        self.earLeft = EarExport(from: anthropometry.earParamsLeft)
        self.earRight = EarExport(from: anthropometry.earParamsRight)
        self.confidence = ConfidenceExport(
            headWidth: confidence.headWidth,
            headDepth: confidence.headDepth,
            earLeft: EarExport(
                d1: confidence.earLeft.d1, d2: confidence.earLeft.d2, d3: confidence.earLeft.d3,
                d5: confidence.earLeft.d5, d7: confidence.earLeft.d7, d8: confidence.earLeft.d8
            ),
            earRight: EarExport(
                d1: confidence.earRight.d1, d2: confidence.earRight.d2, d3: confidence.earRight.d3,
                d5: confidence.earRight.d5, d7: confidence.earRight.d7, d8: confidence.earRight.d8
            )
        )
    }

    func toJSON() throws -> Data {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        return try encoder.encode(self)
    }
}

extension MeasurementExport.EarExport {
    init(from params: EarParameters) {
        self.d1 = params.d1
        self.d2 = params.d2
        self.d3 = params.d3
        self.d5 = params.d5
        self.d7 = params.d7
        self.d8 = params.d8
    }
}

// MARK: - Confidence tracking

struct EarConfidence: Codable {
    var d1: Float
    var d2: Float
    var d3: Float
    var d5: Float
    var d7: Float
    var d8: Float

    static let unknown = EarConfidence(d1: 0, d2: 0, d3: 0, d5: 0, d7: 0, d8: 0)

    static let population = EarConfidence(
        d1: 0.2, d2: 0.2, d3: 0.2,
        d5: 0.2, d7: 0.2, d8: 0.2
    )
}

struct MeasurementConfidence: Codable {
    var headWidth: Float
    var headDepth: Float
    var earLeft: EarConfidence
    var earRight: EarConfidence

    static let unknown = MeasurementConfidence(
        headWidth: 0, headDepth: 0,
        earLeft: .unknown, earRight: .unknown
    )
}

/// Confidence tier for UI color-coding
enum ConfidenceTier {
    case high    // >= 0.8, green
    case medium  // 0.5-0.8, yellow
    case low     // < 0.5, red

    init(value: Float) {
        if value >= 0.8 { self = .high }
        else if value >= 0.5 { self = .medium }
        else { self = .low }
    }
}
