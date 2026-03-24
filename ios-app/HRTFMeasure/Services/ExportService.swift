import Foundation
import CoreImage
import UIKit

/// Handles JSON export and QR code generation for transferring measurements to the desktop app.
enum ExportService {

    /// Generate JSON data from measurements.
    static func generateJSON(
        anthropometry: Anthropometry,
        confidence: MeasurementConfidence
    ) throws -> Data {
        let export = MeasurementExport(from: anthropometry, confidence: confidence)
        return try export.toJSON()
    }

    /// Save JSON to a temporary file and return its URL for sharing.
    static func saveJSONToFile(
        anthropometry: Anthropometry,
        confidence: MeasurementConfidence
    ) throws -> URL {
        let data = try generateJSON(anthropometry: anthropometry, confidence: confidence)
        let filename = "hrtf_measurements_\(dateString()).json"
        let url = FileManager.default.temporaryDirectory.appendingPathComponent(filename)
        try data.write(to: url)
        return url
    }

    /// Generate a QR code image encoding the measurement JSON.
    /// Falls back to a compact format if the full JSON is too large for QR.
    static func generateQRCode(
        anthropometry: Anthropometry
    ) -> UIImage? {
        // Compact format for QR (no confidence, minimal keys)
        let compact: [String: Any] = [
            "v": 1,
            "hw": roundTo2(anthropometry.headWidth),
            "hd": roundTo2(anthropometry.headDepth),
            "el": anthropometry.earParamsLeft.asArray.map { roundTo2($0) },
            "er": anthropometry.earParamsRight.asArray.map { roundTo2($0) }
        ]

        guard let jsonData = try? JSONSerialization.data(withJSONObject: compact),
              let jsonString = String(data: jsonData, encoding: .utf8) else {
            return nil
        }

        guard let filter = CIFilter(name: "CIQRCodeGenerator") else { return nil }
        filter.setValue(jsonString.data(using: .utf8), forKey: "inputMessage")
        filter.setValue("M", forKey: "inputCorrectionLevel")

        guard let ciImage = filter.outputImage else { return nil }

        // Scale up the QR code
        let scale = CGAffineTransform(scaleX: 10, y: 10)
        let scaledImage = ciImage.transformed(by: scale)

        let context = CIContext()
        guard let cgImage = context.createCGImage(scaledImage, from: scaledImage.extent) else {
            return nil
        }

        return UIImage(cgImage: cgImage)
    }

    // MARK: - Private

    private static func dateString() -> String {
        let formatter = DateFormatter()
        formatter.dateFormat = "yyyy-MM-dd_HHmmss"
        return formatter.string(from: Date())
    }

    private static func roundTo2(_ value: Float) -> Float {
        (value * 100).rounded() / 100
    }
}
