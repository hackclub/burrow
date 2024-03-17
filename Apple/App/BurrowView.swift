import SwiftUI

struct BurrowView: View {
    var body: some View {
        NavigationStack {
            VStack {
                NetworkCarouselView()
                Spacer()
                TunnelStatusView()
                TunnelButton()
                    .padding(.bottom)
            }
            .padding()
            .navigationTitle("Networks")
        }
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
