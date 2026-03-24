import SwiftUI

/// Step-by-step scanning flow: Head → Left Ear → Right Ear → Review.
struct ScanFlowView: View {
    @EnvironmentObject var store: MeasurementsStore
    @StateObject private var flowVM = ScanFlowViewModel()

    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                // Progress indicator
                ProgressBar(
                    steps: ScanFlowViewModel.ScanStep.allCases,
                    current: flowVM.currentStep
                )
                .padding()

                // Instruction text
                Text(flowVM.currentStep.instruction)
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
                    .padding(.horizontal)

                Spacer()

                // Current step content
                Group {
                    switch flowVM.currentStep {
                    case .head:
                        HeadScanView(onComplete: { width, depth in
                            store.updateHeadWidth(width, confidence: 0.9)
                            store.updateHeadDepth(depth, confidence: 0.85)
                            flowVM.advanceToNextStep()
                        })
                    case .leftEar:
                        EarCaptureView(side: .left, onComplete: { params in
                            applyEarParams(params, side: .left)
                            flowVM.advanceToNextStep()
                        })
                    case .rightEar:
                        EarCaptureView(side: .right, onComplete: { params in
                            applyEarParams(params, side: .right)
                            flowVM.advanceToNextStep()
                        })
                    case .review:
                        VStack(spacing: 16) {
                            Image(systemName: "checkmark.circle.fill")
                                .font(.system(size: 60))
                                .foregroundStyle(.green)
                            Text("Scan Complete")
                                .font(.title2.bold())
                            Text("Switch to the Results tab to review your measurements.")
                                .foregroundStyle(.secondary)
                        }
                        .onAppear {
                            store.scanState = .reviewingResults
                        }
                    }
                }

                Spacer()

                // Device capability info
                if flowVM.currentStep != .review {
                    DeviceCapabilityBadge(
                        hasLiDAR: flowVM.hasLiDAR,
                        hasTrueDepth: flowVM.hasTrueDepth
                    )
                    .padding(.bottom)
                }
            }
            .navigationTitle("HRTF Measure")
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Reset") {
                        store.reset()
                        flowVM.goToStep(.head)
                    }
                }
            }
        }
    }

    private func applyEarParams(_ params: EarParameters, side: EarCaptureViewModel.EarSide) {
        let macro = params.macroParams
        let concha = params.conchaParams

        switch side {
        case .left:
            store.updateLeftEarMacro(d5: macro.d5, d7: macro.d7, d8: macro.d8, confidence: 0.85)
            store.updateLeftEarConcha(d1: concha.d1, d2: concha.d2, d3: concha.d3, confidence: 0.5)
        case .right:
            store.updateRightEarMacro(d5: macro.d5, d7: macro.d7, d8: macro.d8, confidence: 0.85)
            store.updateRightEarConcha(d1: concha.d1, d2: concha.d2, d3: concha.d3, confidence: 0.5)
        }
    }
}

// MARK: - Progress Bar

struct ProgressBar: View {
    let steps: [ScanFlowViewModel.ScanStep]
    let current: ScanFlowViewModel.ScanStep

    var body: some View {
        HStack(spacing: 4) {
            ForEach(steps, id: \.rawValue) { step in
                VStack(spacing: 4) {
                    Circle()
                        .fill(stepColor(step))
                        .frame(width: 10, height: 10)
                    Text(step.title)
                        .font(.caption2)
                        .foregroundStyle(step == current ? .primary : .secondary)
                }
                .frame(maxWidth: .infinity)

                if step != steps.last {
                    Rectangle()
                        .fill(step.rawValue < current.rawValue ? Color.green : Color.gray.opacity(0.3))
                        .frame(height: 2)
                        .frame(maxWidth: 30)
                }
            }
        }
    }

    private func stepColor(_ step: ScanFlowViewModel.ScanStep) -> Color {
        if step.rawValue < current.rawValue { return .green }
        if step == current { return .blue }
        return .gray.opacity(0.3)
    }
}

// MARK: - Device Capability Badge

struct DeviceCapabilityBadge: View {
    let hasLiDAR: Bool
    let hasTrueDepth: Bool

    var body: some View {
        HStack(spacing: 12) {
            badge("TrueDepth", available: hasTrueDepth)
            badge("LiDAR", available: hasLiDAR)
        }
        .padding(.horizontal)
    }

    private func badge(_ name: String, available: Bool) -> some View {
        HStack(spacing: 4) {
            Image(systemName: available ? "checkmark.circle.fill" : "xmark.circle")
                .foregroundStyle(available ? .green : .red)
                .font(.caption)
            Text(name)
                .font(.caption)
                .foregroundStyle(.secondary)
        }
    }
}
