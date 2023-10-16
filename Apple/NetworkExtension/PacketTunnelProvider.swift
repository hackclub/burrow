import libburrow
import NetworkExtension
import os

class PacketTunnelProvider: NEPacketTunnelProvider {
    let logger = Logger(subsystem: "com.hackclub.burrow", category: "frontend")
    var client: BurrowIpc?
    var osInitialized = false
    override func startTunnel(options: [String: NSObject]?, completionHandler: @escaping (Error?) -> Void) {
        logger.log("Starting tunnel")
        if !osInitialized {
            libburrow.initialize_oslog()
            osInitialized = true
        }
        libburrow.start_srv()
        client = BurrowIpc(logger: logger)
        logger.info("Started server")
        Task {
            do {
                let command = BurrowRequest(id: 0, command: "ServerConfig")
                guard let data = try await client?.request(command, type: Response<BurrowResult<ServerConfigData>>.self)
                else {
                    throw BurrowError.cantParseResult
                }
                let encoded = try JSONEncoder().encode(data.result)
                self.logger.log("Received final data: \(String(decoding: encoded, as: UTF8.self))")
                guard let serverconfig = data.result.Ok else {
                    throw BurrowError.resultIsError
                }
                guard let tunNs = self.generateTunSettings(from: serverconfig) else {
                    throw BurrowError.addrDoesntExist
                }
                try await self.setTunnelNetworkSettings(tunNs)
                self.logger.info("Set remote tunnel address to \(tunNs.tunnelRemoteAddress)")
                completionHandler(nil)
            } catch {
                self.logger.error("An error occurred: \(error)")
                completionHandler(error)
            }
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
    override func stopTunnel(with reason: NEProviderStopReason, completionHandler: @escaping () -> Void) {
        completionHandler()
    }
    override func handleAppMessage(_ messageData: Data, completionHandler: ((Data?) -> Void)?) {
        if let handler = completionHandler {
            handler(messageData)
        }
    }
    override func sleep(completionHandler: @escaping () -> Void) {
        completionHandler()
    }
    override func wake() {
    }
}
