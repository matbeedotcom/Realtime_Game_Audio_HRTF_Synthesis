import SwiftUI

/// Export measurements as JSON file, AirDrop, or QR code for the desktop synthesizer.
struct ExportView: View {
    @EnvironmentObject var store: MeasurementsStore
    @State private var showShareSheet = false
    @State private var showQRCode = false
    @State private var exportURL: URL?
    @State private var errorMessage: String?
    @State private var jsonPreview: String?

    var body: some View {
        NavigationStack {
            List {
                // JSON Preview
                Section("JSON Preview") {
                    if let preview = jsonPreview {
                        Text(preview)
                            .font(.system(.caption, design: .monospaced))
                            .lineLimit(20)
                    } else {
                        Button("Generate Preview") {
                            generatePreview()
                        }
                    }
                }

                // Export options
                Section("Export") {
                    Button {
                        exportAsFile()
                    } label: {
                        Label("Share JSON File", systemImage: "doc.text")
                    }
                    .disabled(!store.anthropometry.isValid)

                    Button {
                        showQRCode = true
                    } label: {
                        Label("Show QR Code", systemImage: "qrcode")
                    }
                    .disabled(!store.anthropometry.isValid)
                }

                // CLI command preview
                Section("CLI Command") {
                    let cmd = cliCommand
                    Text(cmd)
                        .font(.system(.caption, design: .monospaced))
                        .lineLimit(10)
                        .textSelection(.enabled)
                }

                if let error = errorMessage {
                    Section {
                        Label(error, systemImage: "exclamationmark.triangle")
                            .foregroundStyle(.red)
                    }
                }
            }
            .navigationTitle("Export")
            .sheet(isPresented: $showShareSheet) {
                if let url = exportURL {
                    ShareSheet(items: [url])
                }
            }
            .sheet(isPresented: $showQRCode) {
                QRCodeView(anthropometry: store.anthropometry)
            }
            .onAppear {
                generatePreview()
            }
        }
    }

    private func generatePreview() {
        do {
            let data = try store.exportJSON()
            jsonPreview = String(data: data, encoding: .utf8)
        } catch {
            errorMessage = "Failed to generate JSON: \(error.localizedDescription)"
        }
    }

    private func exportAsFile() {
        do {
            let url = try ExportService.saveJSONToFile(
                anthropometry: store.anthropometry,
                confidence: store.confidence
            )
            exportURL = url
            showShareSheet = true
        } catch {
            errorMessage = "Export failed: \(error.localizedDescription)"
        }
    }

    private var cliCommand: String {
        let a = store.anthropometry
        let el = a.earParamsLeft.asArray.map { String(format: "%.2f", $0) }.joined(separator: ",")
        let er = a.earParamsRight.asArray.map { String(format: "%.2f", $0) }.joined(separator: ",")
        return """
        hrtf-synth \\
          --head-width \(String(format: "%.1f", a.headWidth)) \\
          --head-depth \(String(format: "%.1f", a.headDepth)) \\
          --ear-left \(el) \\
          --ear-right \(er) \\
          --output my_hrtf.wav
        """
    }
}

// MARK: - Share Sheet (UIKit wrapper)

struct ShareSheet: UIViewControllerRepresentable {
    let items: [Any]

    func makeUIViewController(context: Context) -> UIActivityViewController {
        UIActivityViewController(activityItems: items, applicationActivities: nil)
    }

    func updateUIViewController(_ uiViewController: UIActivityViewController, context: Context) {}
}
