import SwiftUI

struct TunnelStatusView: View {
    @Environment(\.tunnel)
    var tunnel: any Tunnel

    var body: some View {
        Text(tunnel.status.description)
    }
}

extension TunnelStatus: CustomStringConvertible {
    public var description: String {
        switch self {
        case .unknown:
            "Unknown"
        case .permissionRequired:
            "Permission Required"
        case .disconnected:
            "Disconnected"
        case .disabled:
            "Disabled"
        case .connecting:
            "Connecting…"
        case .connected:
            "Connected"
        case .disconnecting:
            "Disconnecting…"
        case .reasserting:
            "Reasserting…"
        case .invalid:
            "Invalid"
        case .configurationReadWriteFailed:
            "System Error"
        }
    }
}
