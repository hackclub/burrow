import SwiftUI

struct TunnelButton: View {
    @Environment(\.tunnel)
    var tunnel: any Tunnel

    var body: some View {
        if let action = tunnel.action {
            Button {
                tunnel.perform(action)
            } label: {
                Text(action.description)
            }
            .padding(.horizontal)
            .buttonStyle(.floating)
        }
    }
}

extension Tunnel {
    fileprivate var action: TunnelButton.Action? {
        switch status {
        case .permissionRequired, .invalid:
            .enable
        case .disabled, .disconnecting, .disconnected:
            .start
        case .connecting, .connected, .reasserting:
            .stop
        case .unknown, .configurationReadWriteFailed:
            nil
        }
    }
}

extension TunnelButton {
    fileprivate enum Action {
        case enable
        case start
        case stop
    }
}

extension TunnelButton.Action {
    var description: LocalizedStringKey {
        switch self {
        case .enable: "Enable"
        case .start: "Start"
        case .stop: "Stop"
        }
    }
}

extension Tunnel {
    fileprivate func perform(_ action: TunnelButton.Action) {
        switch action {
        case .enable: enable()
        case .start: start()
        case .stop: stop()
        }
    }
}
