import Combine
import NetworkExtension
import SwiftUI

@MainActor
class Tunnel: ObservableObject {
    @Published private(set) var status: Status = .unknown
    @Published private var error: NEVPNError?

    private let bundleIdentifier: String
    private let configure: (NETunnelProviderManager, NETunnelProviderProtocol) -> Void
    private var tasks: [Task<Void, Error>] = []

    private var managers: [NEVPNManager]? {
        didSet { status = currentStatus }
    }

    private var currentStatus: Status {
        guard let managers = managers else {
            guard let error = error else {
                return .unknown
            }

            switch error.code {
            case .configurationReadWriteFailed:
                return .configurationReadWriteFailed
            default:
                return .unknown
            }
        }

        guard let manager = managers.first else {
            return .permissionRequired
        }

        guard manager.isEnabled else {
            return .disabled
        }

        return manager.connection.tunnelStatus
    }

    convenience init(configure: @escaping (NETunnelProviderManager, NETunnelProviderProtocol) -> Void) {
        self.init("com.hackclub.burrow.network", configure: configure)
    }

    init(_ bundleIdentifier: String, configure: @escaping (NETunnelProviderManager, NETunnelProviderProtocol) -> Void) {
        self.bundleIdentifier = bundleIdentifier
        self.configure = configure

        let statusTask = Task {
            for try await _ in NotificationCenter.default.notifications(named: .NEVPNStatusDidChange) {
                status = currentStatus
            }
        }
        let configurationTask = Task {
            for try await _ in NotificationCenter.default.notifications(named: .NEVPNConfigurationChange) {
                await update()
            }
        }
        tasks = [statusTask, configurationTask]
    }

    func update() async {
        do {
            managers = try await NETunnelProviderManager.managers
        } catch let error as NEVPNError {
            self.error = error
        } catch {
            print(error)
        }
    }

    func configure() async throws {
        if managers == nil {
            await update()
        }

        guard let managers = managers else { return }

        if managers.count > 1 {
            try await withThrowingTaskGroup(of: Void.self, returning: Void.self) { group in
                for manager in managers.suffix(from: 1) {
                    group.addTask { try await manager.remove() }
                }
                try await group.waitForAll()
            }
        }

        if managers.isEmpty {
            let manager = NETunnelProviderManager()
            let proto = NETunnelProviderProtocol()
            proto.providerBundleIdentifier = bundleIdentifier
            manager.protocolConfiguration = proto

            configure(manager, proto)
            try await manager.save()
        }
    }

    func start() throws {
        guard let manager = managers?.first else { return }
        try manager.connection.startVPNTunnel()
    }

    func stop() {
        guard let manager = managers?.first else { return }
        manager.connection.stopVPNTunnel()
    }

    deinit {
        tasks.forEach { $0.cancel() }
    }
}

extension NEVPNConnection {
    var tunnelStatus: Tunnel.Status {
        switch status {
        case .connected:
            return .connected(connectedDate!)
        case .connecting:
            return .connecting
        case .disconnecting:
            return .disconnecting
        case .disconnected:
            return .disconnected
        case .reasserting:
            return .reasserting
        case .invalid:
            return .invalid
        @unknown default:
            return .unknown
        }
    }
}
