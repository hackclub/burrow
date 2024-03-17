import BurrowShared
import NetworkExtension

@Observable
class NetworkExtensionTunnel: Tunnel {
    @MainActor private(set) var status: TunnelStatus = .unknown
    private var error: NEVPNError?

    private let logger = Logger.logger(for: Tunnel.self)
    private let bundleIdentifier: String
    private var tasks: [Task<Void, Error>] = []

    // Each manager corresponds to one entry in the Settings app.
    // Our goal is to maintain a single manager, so we create one if none exist and delete any extra.
    private var managers: [NEVPNManager]? {
        didSet { Task { await updateStatus() } }
    }

    private var currentStatus: TunnelStatus {
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

    convenience init() {
        self.init(Constants.networkExtensionBundleIdentifier)
    }

    init(_ bundleIdentifier: String) {
        self.bundleIdentifier = bundleIdentifier

        let center = NotificationCenter.default
        let configurationChanged = Task { [weak self] in
            for try await _ in center.notifications(named: .NEVPNConfigurationChange).map({ _ in () }) {
                await self?.update()
            }
        }
        let statusChanged = Task { [weak self] in
            for try await _ in center.notifications(named: .NEVPNStatusDidChange).map({ _ in () }) {
                await self?.updateStatus()
            }
        }
        tasks = [configurationChanged, statusChanged]

        Task { await update() }
    }

    private func update() async {
        do {
            managers = try await NETunnelProviderManager.managers
            await self.updateStatus()
        } catch let vpnError as NEVPNError {
            error = vpnError
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

        guard managers.isEmpty else { return }

        let manager = NETunnelProviderManager()
        manager.localizedDescription = "Burrow"

        let proto = NETunnelProviderProtocol()
        proto.providerBundleIdentifier = bundleIdentifier
        proto.serverAddress = "hackclub.com"

        manager.protocolConfiguration = proto
        try await manager.save()
    }

    func start() {
        guard let manager = managers?.first else { return }
        Task {
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

    func stop() {
        guard let manager = managers?.first else { return }
        manager.connection.stopVPNTunnel()
    }

    func enable() {
        Task {
            do {
                try await configure()
            } catch {
                logger.error("Failed to enable: \(error)")
            }
        }
    }

    deinit {
        tasks.forEach { $0.cancel() }
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
