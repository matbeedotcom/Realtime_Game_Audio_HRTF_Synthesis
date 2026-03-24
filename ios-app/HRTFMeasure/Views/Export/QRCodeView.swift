import SwiftUI

/// Displays a QR code encoding the anthropometric measurements for quick desktop import.
struct QRCodeView: View {
    let anthropometry: Anthropometry
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            VStack(spacing: 24) {
                Text("Scan this QR code with your desktop app to import measurements.")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
                    .padding(.horizontal)

                if let qrImage = ExportService.generateQRCode(anthropometry: anthropometry) {
                    Image(uiImage: qrImage)
                        .interpolation(.none)
                        .resizable()
                        .scaledToFit()
                        .frame(maxWidth: 280, maxHeight: 280)
                        .padding()
                        .background(Color.white)
                        .clipShape(RoundedRectangle(cornerRadius: 12))
                        .shadow(radius: 4)
                } else {
                    ContentUnavailableView(
                        "QR Generation Failed",
                        systemImage: "qrcode",
                        description: Text("Could not generate QR code from measurements.")
                    )
                }

                // Compact summary
                VStack(alignment: .leading, spacing: 4) {
                    Text("Encoded Values:")
                        .font(.caption.bold())
                    Text(String(format: "Head: %.1f × %.1f cm", anthropometry.headWidth, anthropometry.headDepth))
                        .font(.caption.monospacedDigit())
                    Text("Left ear: \(formatEar(anthropometry.earParamsLeft))")
                        .font(.caption.monospacedDigit())
                    Text("Right ear: \(formatEar(anthropometry.earParamsRight))")
                        .font(.caption.monospacedDigit())
                }
                .foregroundStyle(.secondary)

                Spacer()
            }
            .padding(.top)
            .navigationTitle("QR Code")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Done") { dismiss() }
                }
            }
        }
    }

    private func formatEar(_ params: EarParameters) -> String {
        params.asArray.map { String(format: "%.2f", $0) }.joined(separator: ", ")
    }
}
