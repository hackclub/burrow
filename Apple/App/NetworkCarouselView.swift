import SwiftUI

struct NetworkCarouselView: View {
    var networks: [any Network] = [
        HackClub(id: "1"),
        HackClub(id: "2"),
        WireGuard(id: "4"),
        HackClub(id: "5"),
    ]

    var body: some View {
        ScrollView(.horizontal) {
            LazyHStack {
                ForEach(networks, id: \.id) { network in
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

#if DEBUG
struct NetworkCarouselView_Previews: PreviewProvider {
    static var previews: some View {
        NetworkCarouselView()
    }
}
#endif
