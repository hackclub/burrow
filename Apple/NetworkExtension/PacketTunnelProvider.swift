import libburrow
import NetworkExtension
import OSLog

class PacketTunnelProvider: NEPacketTunnelProvider {
    let logger = Logger(subsystem: "com.hackclub.burrow", category: "General")
    var osInitialized = false
    
    override func startTunnel(options: [String: NSObject]?, completionHandler: @escaping (Error?) -> Void) {
        if(!osInitialized){
            libburrow.initialize_oslog()
            osInitialized=true
        }
        libburrow.spawn_server()
        logger.debug("spawned server")
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
