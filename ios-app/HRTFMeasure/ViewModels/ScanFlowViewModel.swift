import Foundation
import ARKit

/// Orchestrates the multi-step scanning flow: Head → Left Ear → Right Ear → Review.
final class ScanFlowViewModel: ObservableObject {
    @Published var currentStep: ScanStep = .head
    @Published var hasLiDAR: Bool = false
    @Published var hasTrueDepth: Bool = false

    enum ScanStep: Int, CaseIterable {
        case head
        case leftEar
        case rightEar
        case review

        var title: String {
            switch self {
            case .head: return "Head Scan"
            case .leftEar: return "Left Ear"
            case .rightEar: return "Right Ear"
            case .review: return "Review"
            }
        }

        var instruction: String {
            switch self {
            case .head:
                return "Face the front camera and slowly rotate your head left and right."
            case .leftEar:
                return "Turn your head to the right to expose your left ear. Hold the phone 15cm away."
            case .rightEar:
                return "Turn your head to the left to expose your right ear. Hold the phone 15cm away."
            case .review:
                return "Review your measurements below. Tap any value to adjust manually."
            }
        }
    }

    init() {
        checkDeviceCapabilities()
    }

    func checkDeviceCapabilities() {
        hasTrueDepth = ARFaceTrackingConfiguration.isSupported
        hasLiDAR = ARWorldTrackingConfiguration.supportsSceneReconstruction(.mesh)
    }

    func advanceToNextStep() {
        guard let nextIndex = ScanStep.allCases.firstIndex(of: currentStep)
            .map({ ScanStep.allCases.index(after: $0) }),
              nextIndex < ScanStep.allCases.endIndex else { return }
        currentStep = ScanStep.allCases[nextIndex]
    }

    func goToStep(_ step: ScanStep) {
        currentStep = step
    }
}
