import BurrowConfiguration
import NetworkExtension

extension NEPacketTunnelProvider {
    @_dynamicReplacement(for: extensionBundleIdentifier)
    public static var extensionBundleIdentifier: String {
        Constants.networkExtensionBundleIdentifier
    }
}
