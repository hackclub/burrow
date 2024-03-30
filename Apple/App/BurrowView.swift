import AuthenticationServices
import SwiftUI

#if !os(macOS)
struct BurrowView: View {
    @Environment(\.webAuthenticationSession)
    private var webAuthenticationSession

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
