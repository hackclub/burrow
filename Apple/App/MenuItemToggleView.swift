//
//  MenuItemToggleView.swift
//  App
//
//  Created by Thomas Stubblefield on 5/13/23.
//

import SwiftUI
import BurrowShared

struct MenuItemToggleView: View {
    @Environment(\.tunnel)
    var tunnel: Tunnel
    @State private var showAlert = false

    var body: some View {
        VStack {
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
            Button("Add Custom WG Config", action: sncAddCustomnetwork)
        }
        .accessibilityElement(children: .combine)
        .padding(.horizontal, 4)
        .padding(10)
        .frame(minWidth: 300, minHeight: 32, maxHeight: 32)
    }
    
    func sncAddCustomnetwork(){
        Task {
            try await addCustomnetwork()
        }
    }
    
    func addCustomnetwork() async {
        do{
            let networkToml = """
[[peers]]
public_key = "8GaFjVO6c4luCHG4ONO+1bFG8tO+Zz5/Gy+Geht1USM="
preshared_key = "ha7j4BjD49sIzyF9SNlbueK0AMHghlj6+u0G3bzC698="
allowed_ips = ["8.8.8.8/32", "0.0.0.0/0"]
endpoint = "wg.burrow.rs:51820"

[interface]
private_key = "OEPVdomeLTxTIBvv3TYsJRge0Hp9NMiY0sIrhT8OWG8="
address = ["10.13.13.2/24"]
listen_port = 51820
dns = []
"""
            let client = try Client()
            try await client.single_request("AddConfigToml", params: networkToml, type: BurrowResult<AnyResponseData>.self)
            alert("Successs!", isPresented: $showAlert){
                Button("OK", role: .cancel) {}
            }
        } catch {
            
        }
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
