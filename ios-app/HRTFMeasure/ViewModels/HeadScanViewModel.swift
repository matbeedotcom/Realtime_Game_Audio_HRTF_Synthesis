import Foundation
import ARKit
import Combine

/// Manages ARKit face tracking session and extracts head dimensions from the face mesh.
final class HeadScanViewModel: NSObject, ObservableObject {
    @Published var headWidth: Float?
    @Published var headDepth: Float?
    @Published var isScanning = false
    @Published var scanProgress: Float = 0
    @Published var statusMessage = "Position your face in the frame"

    private var session: ARSession?
    private var widthSamples: [Float] = []
    private var depthSamples: [Float] = []
    private let requiredSamples = 30

    /// Head depth correction factor: face mesh depth → full head depth.
    /// Empirically derived from CIPIC database averages.
    private let depthCorrectionFactor: Float = 1.65

    func startScanning() {
        let session = ARSession()
        session.delegate = self
        self.session = session

        let config = ARFaceTrackingConfiguration()
        config.maximumNumberOfTrackedFaces = 1
        session.run(config)

        isScanning = true
        widthSamples = []
        depthSamples = []
        scanProgress = 0
        statusMessage = "Hold still, scanning face..."
    }

    func stopScanning() {
        session?.pause()
        isScanning = false

        if widthSamples.count >= requiredSamples {
            // Use median for robustness
            headWidth = median(widthSamples)
            headDepth = median(depthSamples)
            statusMessage = "Scan complete"
        } else {
            statusMessage = "Scan incomplete — not enough samples"
        }
    }

    private func processFaceMesh(_ faceAnchor: ARFaceAnchor) {
        let vertices = faceAnchor.geometry.vertices

        // Head width: find the maximum left-right extent of the face mesh.
        // The face mesh vertices are in face-local coordinates.
        var minX: Float = .infinity
        var maxX: Float = -.infinity
        var minZ: Float = .infinity
        var maxZ: Float = -.infinity

        for vertex in vertices {
            minX = min(minX, vertex.x)
            maxX = max(maxX, vertex.x)
            minZ = min(minZ, vertex.z)
            maxZ = max(maxZ, vertex.z)
        }

        // Face mesh width in meters → convert to cm.
        // The face mesh covers roughly ear-to-ear, but we add a small correction
        // since the mesh doesn't quite reach the ear canal entrance.
        let meshWidthCm = (maxX - minX) * 100.0
        let earCorrectionCm: Float = 1.5  // approximate gap from mesh edge to ear canal
        let estimatedHeadWidth = meshWidthCm + earCorrectionCm

        // Face mesh depth: nose tip to ear plane, then correct to full head depth.
        let meshDepthCm = (maxZ - minZ) * 100.0
        let estimatedHeadDepth = meshDepthCm * depthCorrectionFactor

        widthSamples.append(estimatedHeadWidth)
        depthSamples.append(estimatedHeadDepth)

        scanProgress = min(1.0, Float(widthSamples.count) / Float(requiredSamples))

        if widthSamples.count >= requiredSamples {
            stopScanning()
        }
    }

    private func median(_ values: [Float]) -> Float {
        let sorted = values.sorted()
        let mid = sorted.count / 2
        if sorted.count.isMultiple(of: 2) {
            return (sorted[mid - 1] + sorted[mid]) / 2.0
        }
        return sorted[mid]
    }
}

// MARK: - ARSessionDelegate

extension HeadScanViewModel: ARSessionDelegate {
    func session(_ session: ARSession, didUpdate anchors: [ARAnchor]) {
        guard isScanning else { return }
        for anchor in anchors {
            if let faceAnchor = anchor as? ARFaceAnchor {
                processFaceMesh(faceAnchor)
            }
        }
    }

    func session(_ session: ARSession, didFailWithError error: Error) {
        statusMessage = "AR session failed: \(error.localizedDescription)"
        isScanning = false
    }
}
