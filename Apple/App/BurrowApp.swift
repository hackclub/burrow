import NetworkExtension
import SwiftUI

@available(macOS 13.0, *)
@main
@MainActor
struct BurrowApp: App {
    // To connect to the App Delegate
    @NSApplicationDelegateAdaptor(AppDelegate.self)
    var delegate
    var body: some Scene {
        WindowGroup(id: "onboarding", content: {
            OnboardingView().frame(width: 1000, height: 600.0).scaledToFill().fixedSize()
        }).windowStyle(.hiddenTitleBar).windowResizability(.contentSize)
    }
}

@available(macOS 13.0, *)
@MainActor
class AppDelegate: NSObject, NSApplicationDelegate {
    static let tunnel = Tunnel { manager, proto in
        proto.serverAddress = "hackclub.com"
        manager.localizedDescription = "Burrow"
    }

    // Verifies app status
    func checkFirstTime() {
        let launchedBefore = UserDefaults.standard.bool(forKey: "launchedBefore")
        if launchedBefore {
            print("Not first launch.")
            closeApp()
        } else {
            print("First launch, setting UserDefault.")
            setVisited()
        }
    }
    
    var statusItem: NSStatusItem?
    var popOver = NSPopover()
    func applicationDidFinishLaunching(_ notification: Notification) {
        checkFirstTime()
        let menuView = MenuView(tunnel: AppDelegate.tunnel)
        // Creating apopOver
        popOver.behavior = .transient
        popOver.animates = true
        popOver.contentViewController = NSViewController()
        popOver.contentViewController?.view = NSHostingView(rootView: menuView)
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        // Safe Check if status Button is Available or not...
        if let menuButton = statusItem?.button {
            let icon = "network.badge.shield.half.filled"
            menuButton.image = NSImage(systemSymbolName: icon, accessibilityDescription: nil)
            menuButton.action = #selector(menuButtonToggle)
        }
    }
    @objc func
    menuButtonToggle() {
        if let menuButton = statusItem?.button {
            self.popOver.show(relativeTo: menuButton.bounds, of: menuButton, preferredEdge: NSRectEdge.minY)
        }
    }
}
