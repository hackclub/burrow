//
//  MenuView.swift
//  App
//
//  Created by Thomas Stubblefield on 5/13/23.
//

import SwiftUI

struct MenuItemToggleView: View {
    var tunnel: Tunnel

    var body: some View {
        HStack {
            Text("Burrow")
                .font(.headline)
            Spacer()
            Toggle("Burrow", isOn: tunnel.isOn)
                .labelsHidden()
                .disabled(tunnel.isDisabled)
                .toggleStyle(.switch)
        }
        .padding(.horizontal, 4)
        .padding(10)
        .frame(minWidth: 300, minHeight: 32, maxHeight: 32)
    }
}

extension Tunnel {
    var isDisabled: Bool {
        switch self.status {
        case .disconnected, .permissionRequired, .connected:
            return false
        case .unknown, .disabled, .connecting, .reasserting, .disconnecting, .invalid, .configurationReadWriteFailed:
            return true
        }
    }

    var isOn: Binding<Bool> {
        Binding {
            switch self.status {
            case .connecting, .reasserting, .connected:
                true
            default:
                false
            }
        } set: { newValue in
            switch (self.status, newValue) {
            case (.permissionRequired, true):
                Task { try await self.configure() }
            case (.disconnected, true):
                try? self.start()
            case (.connected, false):
                self.stop()
            default:
                return
            }
        }
    }
}
