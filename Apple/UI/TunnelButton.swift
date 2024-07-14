import SwiftUI

struct TunnelButton: View {
    @Environment(\.tunnel)
    var tunnel: any Tunnel

    private var action: Action? { tunnel.action }

    var body: some View {
        Button {
            if let action {
                tunnel.perform(action)
            }
        } label: {
            Text(action.description)
        }
        .disabled(action.isDisabled)
        .padding(.horizontal)
        .buttonStyle(.floating)
    }
}

extension Tunnel {
    @MainActor fileprivate var action: TunnelButton.Action? {
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

extension TunnelButton.Action? {
    var description: LocalizedStringKey {
        switch self {
        case .enable: "Enable"
        case .start: "Start"
        case .stop: "Stop"
        case .none: "Start"
        }
    }

    var isDisabled: Bool {
        if case .none = self {
            true
        } else {
            false
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
