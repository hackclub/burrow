import BurrowShared
import libburrow
import NetworkExtension
import os

class PacketTunnelProvider: NEPacketTunnelProvider {
    private let logger = Logger.logger(for: PacketTunnelProvider.self)

    override init() {
        do {
            libburrow.spawnInProcess(socketPath: try Constants.socketURL.path)
        } catch {
            logger.error("Failed to spawn: \(error)")
        }
    }

    override func startTunnel(options: [String: NSObject]? = nil) async throws {
        do {
            let client = try Client()

            let command = BurrowRequest(id: 0, command: "ServerConfig")
            let data = try await client.request(command, type: Response<BurrowResult<ServerConfigData>>.self)

            let encoded = try JSONEncoder().encode(data.result)
            self.logger.log("Received final data: \(String(decoding: encoded, as: UTF8.self))")
            guard let serverconfig = data.result.Ok else {
                throw BurrowError.resultIsError
            }
            guard let tunNs = generateTunSettings(from: serverconfig) else {
                throw BurrowError.addrDoesntExist
            }
            try await self.setTunnelNetworkSettings(tunNs)
            self.logger.info("Set remote tunnel address to \(tunNs.tunnelRemoteAddress)")

            let startRequest = BurrowRequest(
                id: .random(in: (.min)..<(.max)),
                command: BurrowStartRequest(
                    Start: BurrowStartRequest.StartOptions(
                        tun: BurrowStartRequest.TunOptions(
                            name: nil, no_pi: false, tun_excl: false, tun_retrieve: true, address: []
                        )
                    )
                )
            )
            let response = try await client.request(startRequest, type: Response<BurrowResult<String>>.self)
            self.logger.log("Received start server response: \(String(describing: response.result))")
        } catch {
            self.logger.error("Failed to start tunnel: \(error)")
            throw error
        }
    }

    override func stopTunnel(with reason: NEProviderStopReason) async {
        do {
            let client = try Client()
            let command = BurrowRequest(id: 0, command: "Stop")
            let data = try await client.request(command, type: Response<BurrowResult<String>>.self)
            self.logger.log("Stopped client.")
        } catch {
            self.logger.error("Failed to stop tunnel: \(error)")
        }
    }

    private func generateTunSettings(from: ServerConfigData) -> NETunnelNetworkSettings? {
        let cfig = from.ServerConfig
        let nst = NEPacketTunnelNetworkSettings(tunnelRemoteAddress: "1.1.1.1")
        var v4Addresses = [String]()
        var v6Addresses = [String]()
        for addr in cfig.address {
            if IPv4Address(addr) != nil {
                v6Addresses.append(addr)
            }
            if IPv6Address(addr) != nil {
                v4Addresses.append(addr)
            }
        }
        nst.ipv4Settings = NEIPv4Settings(addresses: v4Addresses, subnetMasks: v4Addresses.map { _ in
            "255.255.255.0"
        })
        nst.ipv6Settings = NEIPv6Settings(addresses: v6Addresses, networkPrefixLengths: v6Addresses.map { _ in 64 })
        logger.log("Initialized ipv4 settings: \(nst.ipv4Settings)")
        return nst
    }
}
