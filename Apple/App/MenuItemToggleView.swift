//
//  MenuItemToggleView.swift
//  App
//
//  Created by Thomas Stubblefield on 5/13/23.
//

import SwiftUI

struct MenuItemToggleView: View {
    @Environment(\.tunnel)
    var tunnel: Tunnel

    var body: some View {
        HStack {
            VStack(alignment: .leading) {
                Text("Burrow")
                    .font(.headline)
                Text(tunnel.status.description)
                    .font(.subheadline)
            }
            Spacer()
            Toggle(isOn: tunnel.toggleIsOn) {
            }
                .disabled(tunnel.toggleDisabled)
                .toggleStyle(.switch)
        }
        .accessibilityElement(children: .combine)
        .padding(.horizontal, 4)
        .padding(10)
        .frame(minWidth: 300, minHeight: 32, maxHeight: 32)
    }
}

extension Tunnel {
    fileprivate var toggleDisabled: Bool {
        switch status {
        case .disconnected, .permissionRequired, .connected, .disconnecting:
            false
        case .unknown, .disabled, .connecting, .reasserting, .invalid, .configurationReadWriteFailed:
            true
        }
    }

    var toggleIsOn: Binding<Bool> {
        Binding {
            switch status {
            case .connecting, .reasserting, .connected:
                true
            default:
                false
            }
        } set: { newValue in
            switch (status, newValue) {
            case (.permissionRequired, true):
                enable()
            case (_, true):
                start()
            case (_, false):
                stop()
            }
        }
    }
}
