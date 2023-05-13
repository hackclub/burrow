//
//  MenuView.swift
//  App
//
//  Created by Thomas Stubblefield on 5/13/23.
//

import SwiftUI

struct MenuView: View {
    @State private var isToggled = false
    @ObservedObject var tunnel: Tunnel

    private func start() {
        
        do {
            try tunnel.start()
        } catch {
            print(error)
        }
    }

    private func stop() {
        tunnel.stop()
    }

    private func configure() {
        Task { try await tunnel.configure() }
    }

    var body: some View {
        VStack {
            HStack {
                Text("Burrow")
                    .fontWeight(.bold)

                Spacer()
                Toggle("", isOn: $isToggled)
                    .toggleStyle(SwitchToggleStyle(tint: .blue))
                    .onChange(of: isToggled) { value in
                        if value {
                            start()
                        } else {
                            stop()
                        }
                        print("Toggle value: \(value)")
                    }
            }
            Divider()
            switch tunnel.status {
            case .permissionRequired:
                VStack(alignment: .leading) {
                    Text("Burrow requires additional permissions to function optimally on your machine. Please grant the necessary permissions to ensure smooth operation.")
                        .font(.caption)
                        .truncationMode(.tail)

                    Button("Grant Permissions", action: configure)
                }
            default:

                Text("Burrow is equipped with the necessary permissions to operate seamlessly on your device.")
                .font(.caption)
            }
        }
        .frame(width: 250)
        .padding(16)
        .task { await tunnel.update() }
    }
}
