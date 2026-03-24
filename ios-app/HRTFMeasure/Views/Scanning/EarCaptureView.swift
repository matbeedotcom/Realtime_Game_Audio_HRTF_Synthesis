import SwiftUI

/// Captures ear geometry via LiDAR or guided photo, then runs ML measurement extraction.
struct EarCaptureView: View {
    let side: EarCaptureViewModel.EarSide
    let onComplete: (EarParameters) -> Void

    @StateObject private var viewModel = EarCaptureViewModel()
    @EnvironmentObject var store: MeasurementsStore

    private var sideLabel: String {
        side == .left ? "Left" : "Right"
    }

    var body: some View {
        VStack(spacing: 20) {
            // Capture area
            ZStack {
                RoundedRectangle(cornerRadius: 16)
                    .fill(.black)
                    .aspectRatio(3/4, contentMode: .fit)

                if viewModel.isCapturing {
                    // Ear alignment guide
                    EarGuideOverlay(side: side)

                    // In production: ARViewRepresentable with LiDAR point cloud visualization
                } else if let params = viewModel.measuredParams {
                    MeasuredParamsPreview(params: params, side: sideLabel)
                } else {
                    VStack(spacing: 8) {
                        Image(systemName: "ear")
                            .font(.system(size: 60))
                            .foregroundStyle(.white.opacity(0.5))
                        Text("Tap Start to capture \(sideLabel.lowercased()) ear")
                            .foregroundStyle(.white.opacity(0.7))

                        if viewModel.hasLiDAR {
                            Label("LiDAR available", systemImage: "scope")
                                .font(.caption)
                                .foregroundStyle(.green)
                        } else {
                            Label("Photo mode (no LiDAR)", systemImage: "camera")
                                .font(.caption)
                                .foregroundStyle(.yellow)
                        }
                    }
                }
            }
            .padding(.horizontal)

            // Progress
            if viewModel.isCapturing {
                ProgressView(value: viewModel.captureProgress)
                    .padding(.horizontal)
            }

            Text(viewModel.statusMessage)
                .font(.callout)
                .foregroundStyle(.secondary)

            // Controls
            HStack(spacing: 16) {
                if viewModel.isCapturing {
                    Button("Cancel") {
                        viewModel.stopCapture()
                    }
                    .buttonStyle(.bordered)
                } else if let params = viewModel.measuredParams {
                    Button("Rescan") {
                        viewModel.startCapture(side: side)
                    }
                    .buttonStyle(.bordered)

                    Button("Continue") {
                        // Run concha regression before completing
                        let finalParams = runConchaRegression(params)
                        onComplete(finalParams)
                    }
                    .buttonStyle(.borderedProminent)
                } else {
                    Button("Start Capture") {
                        viewModel.startCapture(side: side)
                    }
                    .buttonStyle(.borderedProminent)
                }
            }
        }
    }

    /// Run ML regression to refine concha estimates using head + macro ear measurements.
    private func runConchaRegression(_ params: EarParameters) -> EarParameters {
        var result = params

        if let prediction = MLService.predictConchaParams(
            headWidth: store.anthropometry.headWidth,
            headDepth: store.anthropometry.headDepth,
            d5: params.d5,
            d7: params.d7,
            d8: params.d8
        ) {
            result.updateConcha(d1: prediction.d1, d2: prediction.d2, d3: prediction.d3)
        }

        return result
    }
}

// MARK: - Ear Guide Overlay

struct EarGuideOverlay: View {
    let side: EarCaptureViewModel.EarSide

    var body: some View {
        ZStack {
            // Ear-shaped target outline
            Ellipse()
                .stroke(.white.opacity(0.5), lineWidth: 2)
                .frame(width: 120, height: 180)
                .offset(x: side == .left ? -10 : 10)

            // Distance indicator
            VStack {
                Spacer()
                Text("~15 cm")
                    .font(.caption)
                    .padding(6)
                    .background(.black.opacity(0.6))
                    .foregroundStyle(.white)
                    .clipShape(Capsule())
                    .padding(.bottom, 20)
            }
        }
    }
}

// MARK: - Measured Params Preview

struct MeasuredParamsPreview: View {
    let params: EarParameters
    let side: String

    var body: some View {
        VStack(spacing: 6) {
            Image(systemName: "checkmark.circle.fill")
                .font(.system(size: 30))
                .foregroundStyle(.green)

            Text("\(side) Ear Measured")
                .font(.headline)
                .foregroundStyle(.white)

            VStack(alignment: .leading, spacing: 2) {
                paramRow("Pinna height (d5)", params.d5)
                paramRow("Pinna width (d7)", params.d7)
                paramRow("Canal to rear (d8)", params.d8)
            }
            .font(.caption)
            .foregroundStyle(.white.opacity(0.8))
        }
    }

    private func paramRow(_ label: String, _ value: Float) -> some View {
        HStack {
            Text(label)
            Spacer()
            Text(String(format: "%.2f cm", value))
                .monospacedDigit()
        }
        .frame(maxWidth: 200)
    }
}
