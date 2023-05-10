import SwiftUI

struct TunnelView: View {
    @ObservedObject var tunnel: Tunnel

    var body: some View {
        VStack {
            Text(verbatim: tunnel.status.description)
            switch tunnel.status {
            case .connected:
                Button("Disconnect", action: stop)
            case .permissionRequired:
                Button("Allow", action: configure)
            case .disconnected:
                Button("Start", action: start)
            default:
                EmptyView()
            }
        }
        .task { await tunnel.update() }
        .padding()
    }

    private func start() {
        try? tunnel.start()
    }

    private func stop() {
        tunnel.stop()
    }

    private func configure() {
        Task { try await tunnel.configure() }
    }
}
