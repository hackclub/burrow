#if os(macOS)
import AppKit
import FluidMenuBarExtra

class AppDelegate: NSObject, NSApplicationDelegate {
    private var menuBarExtra: FluidMenuBarExtra?
    
    func applicationDidFinishLaunching(_ notification: Notification) {
        self.menuBarExtra = FluidMenuBarExtra(title: "Burrow", systemImage: "network.badge.shield.half.filled") {
            ContentView()
        }
    }
}
#endif
