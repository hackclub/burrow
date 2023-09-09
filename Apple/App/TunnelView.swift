import SwiftUI

struct TunnelView: View {
    @ObservedObject var tunnel: Tunnel
    @State var useBurrow = false
    


    var body: some View {
        #if os(iOS)
        Text("Burrow")
            .font(.largeTitle)
            .fontWeight(.heavy)
        VStack {
            switch tunnel.status {
            case .connecting, .disconnecting:
                ProgressView().controlSize(.large).padding()
            case .permissionRequired:
                var useBurrow = false
                Button("Configure VPN", action: configure).buttonStyle(.borderedProminent).tint(.red).padding()
            default:
// for someone else to do: clean up my code and make the toggle VERY large that it's juicy. - R. Ruiz (allthesquares)
                Toggle("", isOn: $useBurrow)
                    .disabled(tunnel.status == .unknown || tunnel.status == .configurationReadWriteFailed || tunnel.status == .invalid)
                    .labelsHidden()
                    .controlSize(.large)
                    .padding()
                    .toggleStyle(SwitchToggleStyle(tint: .red))
                    .onChange(of: useBurrow) { value in
                    if value == true {
                        start()
                    } else {
                        stop()
                    }
                    }
            }
            Text(verbatim: tunnel.status.description)
        }
            .task { await tunnel.update() }
        #else
        EmptyView()
        #endif
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
