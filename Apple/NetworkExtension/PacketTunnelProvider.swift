import BurrowShared
import libburrow
import NetworkExtension
import os

class PacketTunnelProvider: NEPacketTunnelProvider {
    private let logger = Logger.logger(for: PacketTunnelProvider.self)

    override func startTunnel(options: [String: NSObject]? = nil) async throws {
        do {
            libburrow.spawnInProcess(socketPath: try Constants.socketURL.path)

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
                            name: nil, no_pi: false, tun_excl: false, tun_retrieve: true, address: nil
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

    private func generateTunSettings(from: ServerConfigData) -> NETunnelNetworkSettings? {
        let cfig = from.ServerConfig
        guard let addr = cfig.address else {
            return nil
        }
        // Using a makeshift remote tunnel address
        let nst = NEPacketTunnelNetworkSettings(tunnelRemoteAddress: "1.1.1.1")
        nst.ipv4Settings = NEIPv4Settings(addresses: [addr], subnetMasks: ["255.255.255.0"])
        logger.log("Initialized ipv4 settings: \(nst.ipv4Settings)")
        return nst
    }
}
