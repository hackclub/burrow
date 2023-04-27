//
//  burrow_barApp.swift
//  burrow-bar
//
//  Created by Thomas Stubblefield on 4/19/23.
//

import SwiftUI
import FluidMenuBarExtra

@main
struct burrowBarApp: App {
    @NSApplicationDelegateAdaptor private var appDelegate: AppDelegate

    var body: some Scene {
        Settings {
            Text("Burrow")
        }
    }
}
class AppDelegate: NSObject, NSApplicationDelegate {
    private var menuBarExtra: FluidMenuBarExtra?
    
    func applicationDidFinishLaunching(_ notification: Notification) {
        self.menuBarExtra = FluidMenuBarExtra(title: "Burrow", systemImage: "network.badge.shield.half.filled") {
            ContentView()
        }
    }
}
