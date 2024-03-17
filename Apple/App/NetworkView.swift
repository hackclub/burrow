import SwiftUI

struct NetworkView<Content: View>: View {
    var color: Color
    var content: () -> Content

    private var gradient: LinearGradient {
        LinearGradient(
            colors: [
                color.opacity(0.8),
                color
            ],
            startPoint: .init(x: 0.2, y: 0),
            endPoint: .init(x: 0.8, y: 1)
        )
    }

    var body: some View {
        content()
            .frame(maxWidth: .infinity, minHeight: 175, maxHeight: 175)
            .background(
                RoundedRectangle(cornerRadius: 10)
                    .fill(gradient)
                    .background(
                        RoundedRectangle(cornerRadius: 10)
                            .fill(.white)
                    )
            )
            .shadow(color: .black.opacity(0.1), radius: 3.0, x: 0, y: 2)
    }
}

struct AddNetworkView: View {
    var body: some View {
        Text("Add Network")
            .frame(maxWidth: .infinity, minHeight: 175, maxHeight: 175)
            .background(
                RoundedRectangle(cornerRadius: 10)
                    .stroke(style: .init(lineWidth: 2, dash: [6]))
            )
    }
}

extension NetworkView where Content == AnyView {
    init(network: any Network) {
        color = network.backgroundColor
        content = { AnyView(network.label) }
    }
}

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
                AddNetworkView()
            }
            .scrollTargetLayout()
        }
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
