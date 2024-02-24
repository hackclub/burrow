#if os(macOS)
import AppKit
import SwiftUI

@MainActor
@NSApplicationMain
class AppDelegate: NSObject, NSApplicationDelegate {
    private let quitItem: NSMenuItem = {
        let quitItem = NSMenuItem(
            title: "Quit Burrow",
            action: #selector(NSApplication.terminate(_:)),
            keyEquivalent: "q"
        )
        quitItem.target = NSApplication.shared
        quitItem.keyEquivalentModifierMask = .command
        return quitItem
    }()

    private let toggleItem: NSMenuItem = {
        let toggleView = NSHostingView(rootView: MenuItemToggleView())
        toggleView.frame.size = CGSize(width: 300, height: 32)
        toggleView.autoresizingMask = [.width]

        let toggleItem = NSMenuItem()
        toggleItem.view = toggleView
        return toggleItem
    }()

    private lazy var menu: NSMenu = {
        let menu = NSMenu()
        menu.items = [
            toggleItem,
            .separator(),
            quitItem
        ]
        return menu
    }()

    private lazy var statusItem: NSStatusItem = {
        let statusBar = NSStatusBar.system
        let statusItem = statusBar.statusItem(withLength: NSStatusItem.squareLength)
        if let button = statusItem.button {
            button.image = NSImage(systemSymbolName: "network.badge.shield.half.filled", accessibilityDescription: nil)
        }
        return statusItem
    }()

    func applicationDidFinishLaunching(_ notification: Notification) {
        statusItem.menu = menu
    }
}
#endif
