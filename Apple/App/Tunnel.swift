import SwiftUI

protocol Tunnel {
    var status: TunnelStatus { get }

    func start()
    func stop()
    func enable()
}

enum TunnelStatus: Equatable, Hashable {
    case unknown
    case permissionRequired
    case disabled
    case connecting
    case connected(Date)
    case disconnecting
    case disconnected
    case reasserting
    case invalid
    case configurationReadWriteFailed
}

struct TunnelKey: EnvironmentKey {
    static let defaultValue: any Tunnel = NetworkExtensionTunnel()
}

extension EnvironmentValues {
    var tunnel: any Tunnel {
        get { self[TunnelKey.self] }
        set { self[TunnelKey.self] = newValue }
    }
}

#if DEBUG
@Observable
class PreviewTunnel: Tunnel {
    var status: TunnelStatus = .permissionRequired

    func start() {
        status = .connected(.now)
    }
    func stop() {
        status = .disconnected
    }
    func enable() {
        status = .disconnected
    }
}
#endif
