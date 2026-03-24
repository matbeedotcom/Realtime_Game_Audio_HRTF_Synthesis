import SwiftUI

struct ContentView: View {
    @EnvironmentObject var store: MeasurementsStore

    enum Tab {
        case scan, results, export
    }

    @State private var selectedTab: Tab = .scan

    var body: some View {
        TabView(selection: $selectedTab) {
            ScanFlowView()
                .tabItem {
                    Label("Scan", systemImage: "viewfinder")
                }
                .tag(Tab.scan)

            MeasurementResultsView()
                .tabItem {
                    Label("Results", systemImage: "list.bullet.rectangle")
                }
                .tag(Tab.results)

            ExportView()
                .tabItem {
                    Label("Export", systemImage: "square.and.arrow.up")
                }
                .tag(Tab.export)
        }
        .onChange(of: store.scanState) { newState in
            if newState == .reviewingResults {
                selectedTab = .results
            }
        }
    }
}
