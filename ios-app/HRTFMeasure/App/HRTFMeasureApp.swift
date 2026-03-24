import SwiftUI

@main
struct HRTFMeasureApp: App {
    @StateObject private var measurementsStore = MeasurementsStore()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(measurementsStore)
        }
    }
}
