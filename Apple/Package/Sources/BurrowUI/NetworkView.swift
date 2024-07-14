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

extension NetworkView where Content == AnyView {
    init(network: any Network) {
        color = network.backgroundColor
        content = { AnyView(network.label) }
    }
}
