import libburrow
import NetworkExtension
import os

class PacketTunnelProvider: NEPacketTunnelProvider {
    let logger = Logger(subsystem: "com.hackclub.burrow", category: "frontend")
    var client: BurrowIpc?
    var osInitialized = false
    override func startTunnel(options: [String: NSObject]? = nil) async throws {
        logger.log("Starting tunnel")
        if !osInitialized {
            libburrow.initialize_oslog()
            osInitialized = true
        }
        libburrow.start_srv()
        client = BurrowIpc(logger: logger)
        logger.info("Started server")
        do {
            let command = BurrowSingleCommand(id: 0, command: "ServerConfig")
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

            //                let tunFd = self.packetFlow.value(forKeyPath: "socket.fileDescriptor") as! Int;
            //                self.logger.info("Found File Descriptor: \(tunFd)")
            let startCommand = start_req_fd(id: 1)
            guard let data = try await client?.request(startCommand, type: Response<BurrowResult<String>>.self)
            else {
                throw BurrowError.cantParseResult
            }
            let encodedStartRes = try JSONEncoder().encode(data.result)
            self.logger.log("Received start server response: \(String(decoding: encodedStartRes, as: UTF8.self))")
        } catch {
            self.logger.error("An error occurred: \(error)")
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
    override func stopTunnel(with reason: NEProviderStopReason) async {
    }
    override func handleAppMessage(_ messageData: Data) async -> Data? {
        messageData
    }
    override func sleep() async {
    }
    override func wake() {
    }
}
