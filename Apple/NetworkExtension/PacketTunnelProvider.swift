import AsyncAlgorithms
import BurrowConfiguration
import BurrowCore
import libburrow
import NetworkExtension
import os

class PacketTunnelProvider: NEPacketTunnelProvider {
    enum Error: Swift.Error {
        case missingTunnelConfiguration
    }

    private let logger = Logger.logger(for: PacketTunnelProvider.self)

    private var client: TunnelClient {
        get throws { try _client.get() }
    }
    private let _client: Result<TunnelClient, Swift.Error> = Result {
        try TunnelClient.unix(socketURL: Constants.socketURL)
    }

    override init() {
        do {
            libburrow.spawnInProcess(
                socketPath: try Constants.socketURL.path(percentEncoded: false),
                databasePath: try Constants.databaseURL.path(percentEncoded: false)
            )
        } catch {
            logger.error("Failed to spawn networking thread: \(error)")
        }
    }

    override func startTunnel(options: [String: NSObject]? = nil) async throws {
        do {
            let configuration = try await Array(client.tunnelConfiguration(.init()).prefix(1)).first
            guard let settings = configuration?.settings else {
                throw Error.missingTunnelConfiguration
            }
            try await setTunnelNetworkSettings(settings)
            _ = try await client.tunnelStart(.init())
            logger.log("Started tunnel with network settings: \(settings)")
        } catch {
            logger.error("Failed to start tunnel: \(error)")
            throw error
        }
    }

    override func stopTunnel(with reason: NEProviderStopReason) async {
        do {
            _ = try await client.tunnelStop(.init())
            logger.log("Stopped client")
        } catch {
            logger.error("Failed to stop tunnel: \(error)")
        }
    }
}

extension Burrow_TunnelConfigurationResponse {
    fileprivate var settings: NEPacketTunnelNetworkSettings {
        let ipv6Addresses = addresses.filter { IPv6Address($0) != nil }

        let settings = NEPacketTunnelNetworkSettings(tunnelRemoteAddress: "1.1.1.1")
        settings.mtu = NSNumber(value: mtu)
        settings.ipv4Settings = NEIPv4Settings(
            addresses: addresses.filter { IPv4Address($0) != nil },
            subnetMasks: ["255.255.255.0"]
        )
        settings.ipv6Settings = NEIPv6Settings(
            addresses: ipv6Addresses,
            networkPrefixLengths: ipv6Addresses.map { _ in 64 }
        )
        return settings
    }
}
