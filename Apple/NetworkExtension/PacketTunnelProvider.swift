import libburrow
import NetworkExtension
import OSLog

class PacketTunnelProvider: NEPacketTunnelProvider {
    let logger = Logger(subsystem: "com.hackclub.burrow", category: "General")
    
    override func startTunnel(options: [String: NSObject]?, completionHandler: @escaping (Error?) -> Void) {
        libburrow.initialize_oslog()
        let fild = libburrow.retrieve()
        if fild == -1 {
            // Not sure if this is the right way to return an error
            logger.error("Failed to retrieve file descriptor for burrow.")
            let err = NSError(
                domain: "com.hackclub.burrow",
                code: 1_010,
                userInfo: [NSLocalizedDescriptionKey: "Failed to find TunInterface"]
            )
            completionHandler(err)
        }
        logger.info("fd: \(fild)")
        let networkSettings = genNetSec(fild: fild)
        logger.info("Network Settings: - ipv4:\(networkSettings.ipv4Settings) -mtu: \(networkSettings.mtu)")
        completionHandler(nil)
    }

    override func stopTunnel(with reason: NEProviderStopReason, completionHandler: @escaping () -> Void) {
        completionHandler()
    }

    func genNetSec(fild: Int32) -> NEPacketTunnelNetworkSettings {
        logger.debug("getting Network settings with fild \(fild) ...")
        let settings = libburrow.getNetworkSettings(fild)
        logger.debug("genNetSec Called: \n ipv4: \(settings.ipv4_addr) \n netmask: \(settings.ipv4_netmask) \n mtu: \(settings.mtu)")
        let tNetworksettings = TunCrateNetworkSettings(addr: settings.ipv4_addr, netmask: settings.ipv4_netmask, mtu: settings.mtu)
        return tNetworksettings.generateNetworkSettings()
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
