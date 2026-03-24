import SwiftUI

/// Displays all 14 anthropometric measurements with confidence color-coding and manual edit.
struct MeasurementResultsView: View {
    @EnvironmentObject var store: MeasurementsStore
    @State private var editingParam: EditableParam?

    var body: some View {
        NavigationStack {
            List {
                // Head measurements
                Section("Head Dimensions") {
                    MeasurementRow(
                        label: "Head width (ear-to-ear)",
                        value: store.anthropometry.headWidth,
                        unit: "cm",
                        confidence: store.confidence.headWidth,
                        onTap: { editingParam = .headWidth }
                    )
                    MeasurementRow(
                        label: "Head depth (front-to-back)",
                        value: store.anthropometry.headDepth,
                        unit: "cm",
                        confidence: store.confidence.headDepth,
                        onTap: { editingParam = .headDepth }
                    )
                }

                // Left ear
                Section("Left Ear") {
                    earParameterRows(
                        params: store.anthropometry.earParamsLeft,
                        confidence: store.confidence.earLeft,
                        side: .left
                    )
                }

                // Right ear
                Section("Right Ear") {
                    earParameterRows(
                        params: store.anthropometry.earParamsRight,
                        confidence: store.confidence.earRight,
                        side: .right
                    )
                }

                // Validity
                Section {
                    HStack {
                        Image(systemName: store.anthropometry.isValid ? "checkmark.circle.fill" : "exclamationmark.triangle.fill")
                            .foregroundStyle(store.anthropometry.isValid ? .green : .orange)
                        Text(store.anthropometry.isValid ? "All measurements within valid range" : "Some measurements are out of range")
                    }
                }
            }
            .navigationTitle("Measurements")
            .sheet(item: $editingParam) { param in
                MeasurementEditView(param: param, store: store)
            }
        }
    }

    @ViewBuilder
    private func earParameterRows(
        params: EarParameters,
        confidence: EarConfidence,
        side: EarSide
    ) -> some View {
        MeasurementRow(label: "d1 — Cavum concha height", value: params.d1, unit: "cm",
                       confidence: confidence.d1, onTap: { editingParam = .ear(side, .d1) })
        MeasurementRow(label: "d2 — Cymba concha height", value: params.d2, unit: "cm",
                       confidence: confidence.d2, onTap: { editingParam = .ear(side, .d2) })
        MeasurementRow(label: "d3 — Cavum concha width", value: params.d3, unit: "cm",
                       confidence: confidence.d3, onTap: { editingParam = .ear(side, .d3) })
        MeasurementRow(label: "d5 — Pinna height", value: params.d5, unit: "cm",
                       confidence: confidence.d5, onTap: { editingParam = .ear(side, .d5) })
        MeasurementRow(label: "d7 — Pinna width", value: params.d7, unit: "cm",
                       confidence: confidence.d7, onTap: { editingParam = .ear(side, .d7) })
        MeasurementRow(label: "d8 — Canal to rear of pinna", value: params.d8, unit: "cm",
                       confidence: confidence.d8, onTap: { editingParam = .ear(side, .d8) })
    }
}

// MARK: - Measurement Row

struct MeasurementRow: View {
    let label: String
    let value: Float
    let unit: String
    let confidence: Float
    let onTap: () -> Void

    var body: some View {
        Button(action: onTap) {
            HStack {
                Circle()
                    .fill(confidenceColor)
                    .frame(width: 8, height: 8)

                VStack(alignment: .leading, spacing: 2) {
                    Text(label)
                        .font(.subheadline)
                        .foregroundStyle(.primary)
                    Text(String(format: "Confidence: %.0f%%", confidence * 100))
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                }

                Spacer()

                Text(String(format: "%.2f %@", value, unit))
                    .font(.body.monospacedDigit())
                    .foregroundStyle(.primary)

                Image(systemName: "pencil.circle")
                    .foregroundStyle(.secondary)
                    .font(.caption)
            }
        }
    }

    private var confidenceColor: Color {
        switch ConfidenceTier(value: confidence) {
        case .high: return .green
        case .medium: return .yellow
        case .low: return .red
        }
    }
}

// MARK: - Editable Parameter Types

enum EarSide {
    case left, right
}

enum EarParam {
    case d1, d2, d3, d5, d7, d8
}

enum EditableParam: Identifiable {
    case headWidth
    case headDepth
    case ear(EarSide, EarParam)

    var id: String {
        switch self {
        case .headWidth: return "headWidth"
        case .headDepth: return "headDepth"
        case .ear(let side, let param):
            let s = side == .left ? "L" : "R"
            return "ear_\(s)_\(param)"
        }
    }

    var label: String {
        switch self {
        case .headWidth: return "Head Width"
        case .headDepth: return "Head Depth"
        case .ear(let side, let param):
            let s = side == .left ? "Left" : "Right"
            let p: String
            switch param {
            case .d1: p = "d1 (cavum concha height)"
            case .d2: p = "d2 (cymba concha height)"
            case .d3: p = "d3 (cavum concha width)"
            case .d5: p = "d5 (pinna height)"
            case .d7: p = "d7 (pinna width)"
            case .d8: p = "d8 (canal to rear)"
            }
            return "\(s) Ear — \(p)"
        }
    }

    var range: ClosedRange<Float> {
        switch self {
        case .headWidth: return Anthropometry.headWidthRange
        case .headDepth: return Anthropometry.headDepthRange
        case .ear: return EarParameters.paramRange
        }
    }
}
