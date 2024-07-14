import BurrowCore
import NetworkExtension

@Observable
public final class NetworkExtensionTunnel: Tunnel {
    @MainActor public private(set) var status: TunnelStatus = .unknown
    @MainActor private var error: NEVPNError?

    private let logger = Logger.logger(for: Tunnel.self)
    private let bundleIdentifier: String
    private let configurationChanged: Task<Void, Error>
    private let statusChanged: Task<Void, Error>

    // Each manager corresponds to one entry in the Settings app.
    // Our goal is to maintain a single manager, so we create one if none exist and delete any extra.
    @MainActor private var managers: [NEVPNManager]? {
        didSet { Task { await updateStatus() } }
    }

    @MainActor private var currentStatus: TunnelStatus {
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

    public init(bundleIdentifier: String) {
        self.bundleIdentifier = bundleIdentifier

        let center = NotificationCenter.default
        let tunnel: OSAllocatedUnfairLock<NetworkExtensionTunnel?> = .init(initialState: .none)
        configurationChanged = Task {
            for try await _ in center.notifications(named: .NEVPNConfigurationChange) {
                try Task.checkCancellation()
                await tunnel.withLock { $0 }?.update()
            }
        }
        statusChanged = Task {
            for try await _ in center.notifications(named: .NEVPNStatusDidChange) {
                try Task.checkCancellation()
                await tunnel.withLock { $0 }?.updateStatus()
            }
        }
        tunnel.withLock { $0 = self }

        Task { await update() }
    }

    private func update() async {
        do {
            let result = try await NETunnelProviderManager.managers
            await MainActor.run {
                managers = result
                status = currentStatus
            }
            await self.updateStatus()
        } catch let vpnError as NEVPNError {
            await MainActor.run {
                error = vpnError
            }
        } catch {
            logger.error("Failed to update VPN configurations: \(error)")
        }
    }

    private func updateStatus() async {
        await MainActor.run {
            status = currentStatus
        }
    }

    func configure() async throws {
        let managers = try await NETunnelProviderManager.managers
        if managers.count > 1 {
            try await withThrowingTaskGroup(of: Void.self, returning: Void.self) { group in
                for manager in managers.suffix(from: 1) {
                    group.addTask { try await manager.remove() }
                }
                try await group.waitForAll()
            }
        }

        guard managers.isEmpty else { return }

        let manager = NETunnelProviderManager()
        manager.localizedDescription = "Burrow"

        let proto = NETunnelProviderProtocol()
        proto.providerBundleIdentifier = bundleIdentifier
        proto.serverAddress = "hackclub.com"

        manager.protocolConfiguration = proto
        try await manager.save()
    }

    public func start() {
        Task {
            guard let manager = try await NETunnelProviderManager.managers.first else { return }
            do {
                if !manager.isEnabled {
                    manager.isEnabled = true
                    try await manager.save()
                }
                try manager.connection.startVPNTunnel()
            } catch {
                logger.error("Failed to start: \(error)")
            }
        }
    }

    public func stop() {
        Task {
            guard let manager = try await NETunnelProviderManager.managers.first else { return }
            manager.connection.stopVPNTunnel()
        }
    }

    public func enable() {
        Task {
            do {
                try await configure()
            } catch {
                logger.error("Failed to enable: \(error)")
            }
        }
    }

    deinit {
        configurationChanged.cancel()
        statusChanged.cancel()
    }
}

extension NEVPNConnection {
    fileprivate var tunnelStatus: TunnelStatus {
        switch status {
        case .connected:
            .connected(connectedDate!)
        case .connecting:
            .connecting
        case .disconnecting:
            .disconnecting
        case .disconnected:
            .disconnected
        case .reasserting:
            .reasserting
        case .invalid:
            .invalid
        @unknown default:
            .unknown
        }
    }
}
