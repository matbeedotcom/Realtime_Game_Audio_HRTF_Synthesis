import SwiftUI

/// Sheet for manually editing a single measurement value with range validation.
struct MeasurementEditView: View {
    let param: EditableParam
    @ObservedObject var store: MeasurementsStore
    @Environment(\.dismiss) private var dismiss

    @State private var textValue = ""
    @State private var errorMessage: String?

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    TextField("Value (cm)", text: $textValue)
                        .keyboardType(.decimalPad)
                        .onAppear {
                            textValue = String(format: "%.2f", currentValue)
                        }
                } header: {
                    Text(param.label)
                } footer: {
                    Text("Valid range: \(String(format: "%.1f", param.range.lowerBound))–\(String(format: "%.1f", param.range.upperBound)) cm")
                }

                if let error = errorMessage {
                    Section {
                        Label(error, systemImage: "exclamationmark.triangle")
                            .foregroundStyle(.red)
                    }
                }
            }
            .navigationTitle("Edit Measurement")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Save") { save() }
                }
            }
        }
    }

    private var currentValue: Float {
        switch param {
        case .headWidth: return store.anthropometry.headWidth
        case .headDepth: return store.anthropometry.headDepth
        case .ear(let side, let earParam):
            let params = side == .left ? store.anthropometry.earParamsLeft : store.anthropometry.earParamsRight
            switch earParam {
            case .d1: return params.d1
            case .d2: return params.d2
            case .d3: return params.d3
            case .d5: return params.d5
            case .d7: return params.d7
            case .d8: return params.d8
            }
        }
    }

    private func save() {
        guard let value = Float(textValue) else {
            errorMessage = "Please enter a valid number."
            return
        }

        guard param.range.contains(value) else {
            errorMessage = "Value must be between \(String(format: "%.1f", param.range.lowerBound)) and \(String(format: "%.1f", param.range.upperBound)) cm."
            return
        }

        applyValue(value)
        dismiss()
    }

    private func applyValue(_ value: Float) {
        switch param {
        case .headWidth:
            store.anthropometry.headWidth = value
        case .headDepth:
            store.anthropometry.headDepth = value
        case .ear(let side, let earParam):
            switch (side, earParam) {
            case (.left, .d1): store.anthropometry.earParamsLeft.d1 = value
            case (.left, .d2): store.anthropometry.earParamsLeft.d2 = value
            case (.left, .d3): store.anthropometry.earParamsLeft.d3 = value
            case (.left, .d5): store.anthropometry.earParamsLeft.d5 = value
            case (.left, .d7): store.anthropometry.earParamsLeft.d7 = value
            case (.left, .d8): store.anthropometry.earParamsLeft.d8 = value
            case (.right, .d1): store.anthropometry.earParamsRight.d1 = value
            case (.right, .d2): store.anthropometry.earParamsRight.d2 = value
            case (.right, .d3): store.anthropometry.earParamsRight.d3 = value
            case (.right, .d5): store.anthropometry.earParamsRight.d5 = value
            case (.right, .d7): store.anthropometry.earParamsRight.d7 = value
            case (.right, .d8): store.anthropometry.earParamsRight.d8 = value
            }
        }
    }
}
