//
//  burrow_barApp.swift
//  burrow-bar
//
//  Created by Thomas Stubblefield on 4/19/23.
//

import SwiftUI
import NetworkExtension

@main
struct burrowBarApp: App {
    #if os(macOS)
    @NSApplicationDelegateAdaptor private var appDelegate: AppDelegate
    #endif

    var body: some Scene {
        WindowGroup {
            PermissionView()
        }
    }
}

struct PermissionView: View {
    @ObservedObject
    var configuration = NetworkConfiguration()
    
    
    var body: some View {
        VStack {
            Text(verbatim: "Status is \(configuration.status)")
        }
    }
}
