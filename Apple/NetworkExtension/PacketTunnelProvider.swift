import AsyncAlgorithms
import BurrowConfiguration
import BurrowCore
import GRPC
import libburrow
import NetworkExtension
import os

private final class SendableCallbackBox<Callback>: @unchecked Sendable {
    let callback: Callback

    init(_ callback: Callback) {
        self.callback = callback
    }
}

final class PacketTunnelProvider: NEPacketTunnelProvider, @unchecked Sendable {
    enum Error: Swift.Error {
        case missingTunnelConfiguration
    }

    private let logger = Logger.logger(for: PacketTunnelProvider.self)
    private var packetCall: GRPCAsyncBidirectionalStreamingCall<Burrow_TunnelPacket, Burrow_TunnelPacket>?
    private var inboundPacketTask: Task<Void, Never>?
    private var outboundPacketTask: Task<Void, Never>?

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

    override func startTunnel(
        options: [String: NSObject]?,
        completionHandler: @escaping (Swift.Error?) -> Void
    ) {
        let completion = SendableCallbackBox(completionHandler)
        Task {
            do {
                _ = try await client.tunnelStart(.init())
                let configuration = try await Array(client.tunnelConfiguration(.init()).prefix(1)).first
                guard let settings = configuration?.settings else {
                    throw Error.missingTunnelConfiguration
                }
                try await setTunnelNetworkSettings(settings)
                try startPacketBridge()
                logger.log("Started tunnel with network settings: \(settings)")
                completion.callback(nil)
            } catch {
                logger.error("Failed to start tunnel: \(error)")
                stopPacketBridge()
                completion.callback(error)
            }
        }
    }

    override func stopTunnel(
        with reason: NEProviderStopReason,
        completionHandler: @escaping () -> Void
    ) {
        let completion = SendableCallbackBox(completionHandler)
        Task {
            stopPacketBridge()
            do {
                _ = try await client.tunnelStop(.init())
                logger.log("Stopped client")
            } catch {
                logger.error("Failed to stop tunnel: \(error)")
            }
            completion.callback()
        }
    }
}

extension PacketTunnelProvider {
    private func startPacketBridge() throws {
        stopPacketBridge()

        let packetClient = TunnelPacketClient.unix(socketURL: try Constants.socketURL)
        let call = packetClient.makeTunnelPacketsCall()
        self.packetCall = call

        inboundPacketTask = Task { [weak self] in
            guard let self else { return }
            do {
                for try await packet in call.responseStream {
                    let payload = packet.payload
                    self.packetFlow.writePackets(
                        [payload],
                        withProtocols: [Self.protocolNumber(for: payload)]
                    )
                }
            } catch {
                guard !Task.isCancelled else { return }
                self.logger.error("Tunnel packet receive loop failed: \(error)")
            }
        }

        outboundPacketTask = Task { [weak self] in
            guard let self else { return }
            defer { call.requestStream.finish() }
            do {
                while !Task.isCancelled {
                    let packets = await self.readPacketsBatch()
                    for (payload, _) in packets {
                        var packet = Burrow_TunnelPacket()
                        packet.payload = payload
                        try await call.requestStream.send(packet)
                    }
                }
            } catch {
                guard !Task.isCancelled else { return }
                self.logger.error("Tunnel packet send loop failed: \(error)")
            }
        }
    }

    private func stopPacketBridge() {
        inboundPacketTask?.cancel()
        inboundPacketTask = nil
        outboundPacketTask?.cancel()
        outboundPacketTask = nil
        packetCall?.cancel()
        packetCall = nil
    }

    private func readPacketsBatch() async -> [(Data, NSNumber)] {
        await withCheckedContinuation { continuation in
            packetFlow.readPackets { packets, protocols in
                continuation.resume(returning: Array(zip(packets, protocols)))
            }
        }
    }

    private static func protocolNumber(for payload: Data) -> NSNumber {
        guard let version = payload.first.map({ $0 >> 4 }) else {
            return NSNumber(value: AF_INET)
        }
        switch version {
        case 6:
            return NSNumber(value: AF_INET6)
        default:
            return NSNumber(value: AF_INET)
        }
    }
}

extension Burrow_TunnelConfigurationResponse {
    fileprivate var settings: NEPacketTunnelNetworkSettings {
        let parsedAddresses = addresses.compactMap(ParsedTunnelAddress.init(rawValue:))
        let ipv4Addresses = parsedAddresses.compactMap(\.ipv4Address)
        let ipv6Addresses = parsedAddresses.compactMap(\.ipv6Address)
        let parsedRoutes = routes.compactMap(ParsedTunnelRoute.init(rawValue:))
        var ipv4Routes = parsedRoutes.compactMap(\.ipv4Route)
        var ipv6Routes = parsedRoutes.compactMap(\.ipv6Route)
        if includeDefaultRoute {
            ipv4Routes.append(.default())
            ipv6Routes.append(.default())
        }

        let settings = NEPacketTunnelNetworkSettings(tunnelRemoteAddress: "1.1.1.1")
        settings.mtu = NSNumber(value: mtu)
        if !ipv4Addresses.isEmpty {
            let ipv4Settings = NEIPv4Settings(
                addresses: ipv4Addresses.map(\.address),
                subnetMasks: ipv4Addresses.map(\.subnetMask)
            )
            if !ipv4Routes.isEmpty {
                ipv4Settings.includedRoutes = ipv4Routes
            }
            settings.ipv4Settings = ipv4Settings
        }
        if !ipv6Addresses.isEmpty {
            let ipv6Settings = NEIPv6Settings(
                addresses: ipv6Addresses.map(\.address),
                networkPrefixLengths: ipv6Addresses.map(\.prefixLength)
            )
            if !ipv6Routes.isEmpty {
                ipv6Settings.includedRoutes = ipv6Routes
            }
            settings.ipv6Settings = ipv6Settings
        }
        if !dnsServers.isEmpty {
            let dnsSettings = NEDNSSettings(servers: dnsServers)
            if !searchDomains.isEmpty {
                dnsSettings.matchDomains = searchDomains
            }
            settings.dnsSettings = dnsSettings
        }
        return settings
    }
}

private struct ParsedTunnelAddress {
    struct IPv4AddressSetting {
        let address: String
        let subnetMask: String
    }

    struct IPv6AddressSetting {
        let address: String
        let prefixLength: NSNumber
    }

    let ipv4Address: IPv4AddressSetting?
    let ipv6Address: IPv6AddressSetting?

    init?(rawValue: String) {
        let components = rawValue.split(separator: "/", maxSplits: 1).map(String.init)
        let address = components.first?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        guard !address.isEmpty else {
            return nil
        }

        let prefix = components.count == 2 ? Int(components[1]) : nil
        if IPv4Address(address) != nil {
            let prefixLength = prefix ?? 32
            guard (0 ... 32).contains(prefixLength) else {
                return nil
            }
            ipv4Address = IPv4AddressSetting(
                address: address,
                subnetMask: Self.ipv4SubnetMask(prefixLength: prefixLength)
            )
            ipv6Address = nil
            return
        }

        if IPv6Address(address) != nil {
            let prefixLength = prefix ?? 128
            guard (0 ... 128).contains(prefixLength) else {
                return nil
            }
            ipv4Address = nil
            ipv6Address = IPv6AddressSetting(
                address: address,
                prefixLength: NSNumber(value: prefixLength)
            )
            return
        }

        return nil
    }

    private static func ipv4SubnetMask(prefixLength: Int) -> String {
        guard prefixLength > 0 else {
            return "0.0.0.0"
        }
        let mask = UInt32.max << (32 - prefixLength)
        let octets = [
            (mask >> 24) & 0xff,
            (mask >> 16) & 0xff,
            (mask >> 8) & 0xff,
            mask & 0xff,
        ]
        return octets.map(String.init).joined(separator: ".")
    }
}

private struct ParsedTunnelRoute {
    let ipv4Route: NEIPv4Route?
    let ipv6Route: NEIPv6Route?

    init?(rawValue: String) {
        let components = rawValue.split(separator: "/", maxSplits: 1).map(String.init)
        let address = components.first?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        guard !address.isEmpty else {
            return nil
        }

        let prefix = components.count == 2 ? Int(components[1]) : nil
        if IPv4Address(address) != nil {
            let prefixLength = prefix ?? 32
            guard (0 ... 32).contains(prefixLength) else {
                return nil
            }
            ipv4Route = NEIPv4Route(
                destinationAddress: address,
                subnetMask: Self.ipv4SubnetMask(prefixLength: prefixLength)
            )
            ipv6Route = nil
            return
        }

        if IPv6Address(address) != nil {
            let prefixLength = prefix ?? 128
            guard (0 ... 128).contains(prefixLength) else {
                return nil
            }
            ipv4Route = nil
            ipv6Route = NEIPv6Route(
                destinationAddress: address,
                networkPrefixLength: NSNumber(value: prefixLength)
            )
            return
        }

        return nil
    }

    private static func ipv4SubnetMask(prefixLength: Int) -> String {
        var mask = UInt32.max << (32 - prefixLength)
        if prefixLength == 0 {
            mask = 0
        }
        let octets = [
            String((mask >> 24) & 0xff),
            String((mask >> 16) & 0xff),
            String((mask >> 8) & 0xff),
            String(mask & 0xff),
        ]
        return octets.joined(separator: ".")
    }
}
