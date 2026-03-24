import SwiftUI
import ARKit

/// ARKit-based head scanning view that extracts head width and depth.
struct HeadScanView: View {
    let onComplete: (Float, Float) -> Void

    @StateObject private var viewModel = HeadScanViewModel()

    var body: some View {
        VStack(spacing: 20) {
            // AR Camera preview placeholder
            ZStack {
                RoundedRectangle(cornerRadius: 16)
                    .fill(.black)
                    .aspectRatio(3/4, contentMode: .fit)

                if viewModel.isScanning {
                    // Face alignment overlay
                    Image(systemName: "face.dashed")
                        .font(.system(size: 80))
                        .foregroundStyle(.white.opacity(0.5))

                    // In production, this would be replaced with an ARViewRepresentable
                    // showing the actual camera feed with face mesh overlay
                } else if let width = viewModel.headWidth, let depth = viewModel.headDepth {
                    VStack(spacing: 8) {
                        Image(systemName: "checkmark.circle.fill")
                            .font(.system(size: 40))
                            .foregroundStyle(.green)
                        Text(String(format: "Width: %.1f cm", width))
                            .foregroundStyle(.white)
                        Text(String(format: "Depth: %.1f cm", depth))
                            .foregroundStyle(.white)
                    }
                } else {
                    VStack(spacing: 8) {
                        Image(systemName: "face.smiling")
                            .font(.system(size: 60))
                            .foregroundStyle(.white.opacity(0.5))
                        Text("Tap Start to begin head scan")
                            .foregroundStyle(.white.opacity(0.7))
                    }
                }
            }
            .padding(.horizontal)

            // Progress
            if viewModel.isScanning {
                ProgressView(value: viewModel.scanProgress)
                    .padding(.horizontal)
            }

            Text(viewModel.statusMessage)
                .font(.callout)
                .foregroundStyle(.secondary)

            // Controls
            HStack(spacing: 16) {
                if viewModel.isScanning {
                    Button("Cancel") {
                        viewModel.stopScanning()
                    }
                    .buttonStyle(.bordered)
                } else if let width = viewModel.headWidth, let depth = viewModel.headDepth {
                    Button("Rescan") {
                        viewModel.startScanning()
                    }
                    .buttonStyle(.bordered)

                    Button("Continue") {
                        onComplete(width, depth)
                    }
                    .buttonStyle(.borderedProminent)
                } else {
                    Button("Start Scan") {
                        viewModel.startScanning()
                    }
                    .buttonStyle(.borderedProminent)
                }
            }
        }
    }
}
