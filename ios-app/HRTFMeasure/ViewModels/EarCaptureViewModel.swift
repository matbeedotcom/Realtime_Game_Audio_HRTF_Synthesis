import Foundation
import ARKit
import Combine

/// Captures ear geometry using LiDAR point cloud or photo-based fallback.
final class EarCaptureViewModel: NSObject, ObservableObject {
    enum EarSide {
        case left, right
    }

    enum CaptureMode {
        case lidar
        case photo
    }

    @Published var isCapturing = false
    @Published var captureProgress: Float = 0
    @Published var statusMessage = "Position your ear in the frame"
    @Published var capturedPoints: [SIMD3<Float>] = []
    @Published var measuredParams: EarParameters?
    @Published var captureMode: CaptureMode = .lidar

    var earSide: EarSide = .left
    private var session: ARSession?
    private var pointCloudFrames: [[SIMD3<Float>]] = []
    private let requiredFrames = 20

    var hasLiDAR: Bool {
        ARWorldTrackingConfiguration.supportsSceneReconstruction(.mesh)
    }

    func startCapture(side: EarSide) {
        self.earSide = side
        captureMode = hasLiDAR ? .lidar : .photo
        pointCloudFrames = []
        captureProgress = 0

        if captureMode == .lidar {
            startLiDARCapture()
        } else {
            statusMessage = "Take a photo of your \(side == .left ? "left" : "right") ear with a reference card"
        }
    }

    // MARK: - LiDAR Capture

    private func startLiDARCapture() {
        let session = ARSession()
        session.delegate = self
        self.session = session

        let config = ARWorldTrackingConfiguration()
        config.sceneReconstruction = .mesh
        config.frameSemantics = .sceneDepth
        session.run(config)

        isCapturing = true
        statusMessage = "Hold phone ~15cm from ear, move slowly around it"
    }

    func stopCapture() {
        session?.pause()
        isCapturing = false

        if !pointCloudFrames.isEmpty {
            processPointCloud()
        }
    }

    private func processPointCloud() {
        // Merge all captured point cloud frames
        let allPoints = pointCloudFrames.flatMap { $0 }
        capturedPoints = allPoints

        // Extract ear measurements from point cloud using geometric analysis
        let params = GeometryService.extractEarMeasurements(from: allPoints)
        measuredParams = params
        statusMessage = "Capture complete"
    }

    // MARK: - Photo Capture (fallback)

    func processEarPhoto(_ imageData: Data) {
        statusMessage = "Processing ear photo..."
        // Photo-based measurement will use Vision framework + CoreML landmark detection
        // This is a placeholder for the ML pipeline integration
        statusMessage = "Photo processing complete"
    }
}

// MARK: - ARSessionDelegate

extension EarCaptureViewModel: ARSessionDelegate {
    func session(_ session: ARSession, didUpdate frame: ARFrame) {
        guard isCapturing, captureMode == .lidar else { return }

        // Extract depth data from the frame
        guard let depthMap = frame.sceneDepth?.depthMap else { return }

        let width = CVPixelBufferGetWidth(depthMap)
        let height = CVPixelBufferGetHeight(depthMap)

        CVPixelBufferLockBaseAddress(depthMap, .readOnly)
        defer { CVPixelBufferUnlockBaseAddress(depthMap, .readOnly) }

        guard let baseAddress = CVPixelBufferGetBaseAddress(depthMap) else { return }
        let depthPointer = baseAddress.assumingMemoryBound(to: Float32.self)

        // Sample points from the depth map center region (where the ear should be)
        var framePoints: [SIMD3<Float>] = []
        let camera = frame.camera
        let intrinsics = camera.intrinsics

        let centerX = width / 2
        let centerY = height / 2
        let roiSize = min(width, height) / 3

        for y in stride(from: centerY - roiSize/2, to: centerY + roiSize/2, by: 4) {
            for x in stride(from: centerX - roiSize/2, to: centerX + roiSize/2, by: 4) {
                guard x >= 0, x < width, y >= 0, y < height else { continue }
                let depth = depthPointer[y * width + x]
                guard depth > 0.05, depth < 0.5 else { continue }  // 5-50cm range

                // Unproject 2D pixel + depth to 3D point
                let fx = intrinsics[0][0]
                let fy = intrinsics[1][1]
                let cx = intrinsics[2][0]
                let cy = intrinsics[2][1]

                let xWorld = (Float(x) - cx) * depth / fx
                let yWorld = (Float(y) - cy) * depth / fy
                let zWorld = depth

                let localPoint = SIMD3<Float>(xWorld, yWorld, zWorld)
                let worldPoint = camera.transform * SIMD4<Float>(localPoint, 1.0)
                framePoints.append(SIMD3<Float>(worldPoint.x, worldPoint.y, worldPoint.z))
            }
        }

        if !framePoints.isEmpty {
            pointCloudFrames.append(framePoints)
            captureProgress = min(1.0, Float(pointCloudFrames.count) / Float(requiredFrames))

            if pointCloudFrames.count >= requiredFrames {
                stopCapture()
            }
        }
    }
}
