import AuthenticationServices
import SwiftUI
import BurrowShared

#if !os(macOS)
struct BurrowView: View {
    @Environment(\.webAuthenticationSession)
    private var webAuthenticationSession
    @State private var rpcClient: Client?
    @State private var showAlert = false

    var body: some View {
        NavigationStack {
            VStack {
                HStack {
                    Text("Networks")
                        .font(.largeTitle)
                        .fontWeight(.bold)
                    Spacer()
                    Menu {
                        Button("Hack Club", action: addHackClubNetwork)
                        Button("WireGuard", action: addWireGuardNetwork)
                        Button("Custom", action: sncAddCustomnetwork)
                    } label: {
                        Image(systemName: "plus.circle.fill")
                            .font(.title)
                            .accessibilityLabel("Add")
                    }
                }
                .padding(.top)
                NetworkCarouselView()
                Spacer()
                TunnelStatusView()
                TunnelButton()
                    .padding(.bottom)
            }
            .padding()
            .handleOAuth2Callback()
        }
    }
    
    private func addHackClubNetwork() {
        Task {
            try await authenticateWithSlack()
        }
    }

    private func addWireGuardNetwork() {
    }
    private func getClient() throws -> Client {
        if self.rpcClient == nil {
            let client = try Client()
            self.rpcClient = client
        }
        return self.rpcClient!
    }
    private func sncAddCustomnetwork() {
        Task {
            try await addCustomnetwork()
        }
    }
    private func addCustomnetwork() async {
        do {
            let networkToml = ""
            let client = try getClient()
            try await client.single_request("AddConfigToml", params: networkToml, type: BurrowResult<AnyResponseData>.self)
            alert("Successs!", isPresented: $showAlert){
                Button("OK", role: .cancel) {}
            }
            
        } catch {
            
        }
    }

    private func authenticateWithSlack() async throws {
        guard
            let authorizationEndpoint = URL(string: "https://slack.com/openid/connect/authorize"),
            let tokenEndpoint = URL(string: "https://slack.com/api/openid.connect.token"),
            let redirectURI = URL(string: "https://burrow.rs/callback/oauth2") else { return }
        let session = OAuth2.Session(
            authorizationEndpoint: authorizationEndpoint,
            tokenEndpoint: tokenEndpoint,
            redirectURI: redirectURI,
            scopes: ["openid", "profile"],
            clientID: "2210535565.6884042183125",
            clientSecret: "2793c8a5255cae38830934c664eeb62d"
        )
        let response = try await session.authorize(webAuthenticationSession)
    }
}

#if DEBUG
struct NetworkView_Previews: PreviewProvider {
    static var previews: some View {
        BurrowView()
            .environment(\.tunnel, PreviewTunnel())
    }
}
#endif
#endif
