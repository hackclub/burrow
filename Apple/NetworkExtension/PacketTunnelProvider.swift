import BurrowShared
import libburrow
import NetworkExtension
import os

class PacketTunnelProvider: NEPacketTunnelProvider {
    private let logger = Logger.logger(for: PacketTunnelProvider.self)
    private var client: Client?

    override init() {
        do {
            libburrow.spawnInProcess(
                socketPath: try Constants.socketURL.path(percentEncoded: false),
                dbPath: try Constants.dbURL.path(percentEncoded: false)
            )
        } catch {
            logger.error("Failed to spawn: \(error)")
        }
    }

    override func startTunnel(options: [String: NSObject]? = nil) async throws {
        do {
            let client = try Client()
            self.client = client
            register_events(client)

            _ = try await self.loadTunSettings()
            let startRequest = Start(
                tun: Start.TunOptions(
                    name: nil, no_pi: false, tun_excl: false, tun_retrieve: true, address: []
                )
            )
            let response = try await client.request(startRequest, type: BurrowResult<AnyResponseData>.self)
            self.logger.log("Received start server response: \(String(describing: response))")
        } catch {
            self.logger.error("Failed to start tunnel: \(error)")
            throw error
        }
    }

    override func stopTunnel(with reason: NEProviderStopReason) async {
        do {
            let client = try Client()
            _ = try await client.single_request("Stop", type: BurrowResult<AnyResponseData>.self)
            self.logger.log("Stopped client.")
        } catch {
            self.logger.error("Failed to stop tunnel: \(error)")
        }
    }
    func loadTunSettings() async throws -> ServerConfig {
        guard let client = self.client else {
            throw BurrowError.noClient
        }
        let srvConfig = try await client.single_request("ServerConfig", type: BurrowResult<ServerConfig>.self)
        guard let serverconfig = srvConfig.Ok else {
            throw BurrowError.resultIsError
        }
        guard let tunNs = generateTunSettings(from: serverconfig) else {
            throw BurrowError.addrDoesntExist
        }
        try await self.setTunnelNetworkSettings(tunNs)
        self.logger.info("Set remote tunnel address to \(tunNs.tunnelRemoteAddress)")
        return serverconfig
    }
    private func generateTunSettings(from: ServerConfig) -> NETunnelNetworkSettings? {
        // Using a makeshift remote tunnel address
        let nst = NEPacketTunnelNetworkSettings(tunnelRemoteAddress: "1.1.1.1")
        var v4Addresses = [String]()
        var v6Addresses = [String]()
        for addr in from.address {
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
    func register_events(_ client: Client) {
        client.on_event(.ConfigChange) { (cfig: ServerConfig) in
            self.logger.info("Config Change Notification: \(String(describing: cfig))")
            self.setTunnelNetworkSettings(self.generateTunSettings(from: cfig))
            self.logger.info("Updated Tunnel Network Settings.")
        }
    }
}
