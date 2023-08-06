import libburrow
import NetworkExtension
import OSLog

class PacketTunnelProvider: NEPacketTunnelProvider {
    let logger = Logger(subsystem: "com.hackclub.burrow", category: "General")
    override func startTunnel(options: [String: NSObject]?, completionHandler: @escaping (Error?) -> Void) {
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
