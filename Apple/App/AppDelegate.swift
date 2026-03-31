#if os(macOS)
import AppKit
import BurrowUI
import SwiftUI

@main
@MainActor
class AppDelegate: NSObject, NSApplicationDelegate {
    private var windowController: NSWindowController?

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

    private lazy var openItem: NSMenuItem = {
        let item = NSMenuItem(
            title: "Open Burrow",
            action: #selector(openWindow),
            keyEquivalent: "o"
        )
        item.target = self
        item.keyEquivalentModifierMask = .command
        return item
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
            openItem,
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

    @objc
    private func openWindow() {
        if let window = windowController?.window {
            window.makeKeyAndOrderFront(nil)
            NSApplication.shared.activate(ignoringOtherApps: true)
            return
        }

        let contentView = BurrowView()
        let hostingController = NSHostingController(rootView: contentView)
        let window = NSWindow(contentViewController: hostingController)
        window.title = "Burrow"
        window.setContentSize(NSSize(width: 820, height: 720))
        window.styleMask.insert([.titled, .closable, .miniaturizable, .resizable])
        window.center()

        let controller = NSWindowController(window: window)
        controller.shouldCascadeWindows = true
        controller.showWindow(nil)
        windowController = controller
        NSApplication.shared.activate(ignoringOtherApps: true)
    }
}
#endif
