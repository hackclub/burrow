import SwiftUI

struct FloatingButtonStyle: ButtonStyle {
    static let duration = 0.08

    var color: Color
    var cornerRadius: CGFloat

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.headline)
            .foregroundColor(.white)
            .frame(minHeight: 48)
            .padding(.horizontal)
            .background(
                RoundedRectangle(cornerRadius: cornerRadius)
                    .fill(
                        LinearGradient(
                            colors: [
                                configuration.isPressed ? color.opacity(0.9) : color.opacity(0.9),
                                configuration.isPressed ? color.opacity(0.9) : color
                            ],
                            startPoint: .init(x: 0.2, y: 0),
                            endPoint: .init(x: 0.8, y: 1)
                        )
                    )
                    .background(
                        RoundedRectangle(cornerRadius: cornerRadius)
                            .fill(configuration.isPressed ? .black : .white)
                    )
            )
            .shadow(color: .black.opacity(configuration.isPressed ? 0.0 : 0.1), radius: 2.5, x: 0, y: 2)
            .scaleEffect(configuration.isPressed ? 0.975 : 1.0)
            .padding(.bottom, 2)
            .animation(
                configuration.isPressed ? .easeOut(duration: Self.duration) : .easeIn(duration: Self.duration),
                value: configuration.isPressed
            )
    }
}

extension ButtonStyle where Self == FloatingButtonStyle {
    static var floating: FloatingButtonStyle {
        floating()
    }

    static func floating(color: Color = .accentColor, cornerRadius: CGFloat = 10) -> FloatingButtonStyle {
        FloatingButtonStyle(color: color, cornerRadius: cornerRadius)
    }
}
