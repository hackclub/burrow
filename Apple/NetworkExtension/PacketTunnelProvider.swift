import NetworkExtension
import libburrow
import OSLog


class PacketTunnelProvider: NEPacketTunnelProvider {
    let logger = Logger(subsystem: "com.hackclub.burrow", category: "General")
    override func startTunnel(options: [String: NSObject]?, completionHandler: @escaping (Error?) -> Void) {
        let fd = libburrow.start()
        logger.info("fd: \(fd)")
        completionHandler(nil)
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
