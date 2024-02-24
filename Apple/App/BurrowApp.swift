import SwiftUI

#if !os(macOS)
@MainActor
@main
struct BurrowApp: App {
    var body: some Scene {
        WindowGroup {
            BurrowView()
        }
    }
}
#endif
