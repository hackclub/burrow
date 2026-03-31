import SwiftUI

struct NetworkCarouselView: View {
    var networks: [NetworkCardModel]

    var body: some View {
        Group {
            if networks.isEmpty {
                ContentUnavailableView(
                    "No Networks Yet",
                    systemImage: "network.slash",
                    description: Text("Add a WireGuard network, or save a Tailnet account so Burrow can store a managed network when the daemon is reachable.")
                )
                .frame(maxWidth: .infinity, minHeight: 175)
            } else {
                ScrollView(.horizontal) {
                    LazyHStack {
                        ForEach(networks) { network in
                            NetworkView(network: network)
                                .containerRelativeFrame(.horizontal, count: 10, span: 7, spacing: 0, alignment: .center)
                                .scrollTransition(.interactive, axis: .horizontal) { content, phase in
                                    content
                                        .scaleEffect(1.0 - abs(phase.value) * 0.1)
                                }
                        }
                    }
                }
                .scrollTargetLayout()
                .scrollClipDisabled()
                .scrollIndicators(.hidden)
                .defaultScrollAnchor(.center)
                .scrollTargetBehavior(.viewAligned)
                .containerRelativeFrame(.horizontal)
            }
        }
    }
}

#if DEBUG
struct NetworkCarouselView_Previews: PreviewProvider {
    static var previews: some View {
        NetworkCarouselView(networks: [WireGuardCard(id: 1, detail: "10.13.13.2/24 · wg.burrow.rs:51820").card])
    }
}
#endif
