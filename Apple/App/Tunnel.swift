import Combine
import NetworkExtension
import SwiftUI

@Observable class Tunnel {
    private(set) var status: Status = .unknown
    private var error: NEVPNError?

    private let bundleIdentifier: String
    private let configure: (NETunnelProviderManager, NETunnelProviderProtocol) -> Void
    private var tasks: [Task<Void, Error>] = []

    // Each manager corresponds to one entry in the Settings app.
    // Our goal is to maintain a single manager, so we create one if none exist and delete extra if there are any.
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

        listenForUpdates()
        Task { await update() }
    }

    private func listenForUpdates() {
        let center = NotificationCenter.default
        let statusTask = Task {
            for try await _ in center.notifications(named: .NEVPNStatusDidChange).map({ _ in () }) {
                status = currentStatus
            }
        }
        let configurationTask = Task {
            for try await _ in center.notifications(named: .NEVPNConfigurationChange).map({ _ in () }) {
                await update()
            }
        }
        tasks = [statusTask, configurationTask]
    }

    private func update() async {
        do {
            let updated = try await NETunnelProviderManager.managers
            await MainActor.run { managers = updated }
        } catch let vpnError as NEVPNError {
            error = vpnError
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
            configure(manager, proto)

            manager.protocolConfiguration = proto
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
        for task in tasks {
            task.cancel()
        }
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
