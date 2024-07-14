#if !os(macOS)
import BurrowUI
import SwiftUI

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
