import SwiftUI

@main
@MainActor
struct BurrowApp: App {
    static let tunnel = Tunnel { manager, proto in
        proto.serverAddress = "hackclub.com"
        manager.localizedDescription = "Burrow"
    }

    #if os(macOS)
    @NSApplicationDelegateAdaptor(AppDelegate.self)
    var delegate
    #endif

    var body: some Scene {
        WindowGroup {
            TunnelView(tunnel: Self.tunnel)
        }
    }
}
