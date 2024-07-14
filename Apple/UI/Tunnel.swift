import BurrowConfiguration
import NetworkExtension
import SwiftUI

protocol Tunnel: Sendable {
    @MainActor var status: TunnelStatus { get }

    func start()
    func stop()
    func enable()
}

public enum TunnelStatus: Sendable, Equatable, Hashable {
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
    static var defaultValue: any Tunnel {
        NetworkExtensionTunnel(bundleIdentifier: Constants.networkExtensionBundleIdentifier)
    }
}

extension EnvironmentValues {
    var tunnel: any Tunnel {
        get { self[TunnelKey.self] }
        set { self[TunnelKey.self] = newValue }
    }
}

#if DEBUG
@Observable
@MainActor
final class PreviewTunnel: Tunnel {
    private(set) var status: TunnelStatus = .permissionRequired

    nonisolated func start() {
        set(.connected(.now))
    }

    nonisolated func stop() {
        set(.disconnected)
    }

    nonisolated func enable() {
        set(.disconnected)
    }

    nonisolated private func set(_ status: TunnelStatus) {
        Task { @MainActor in self.status = status }
    }
}
#endif
