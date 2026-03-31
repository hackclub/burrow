import BurrowCore
import Foundation
import SwiftUI

struct WireGuardCard {
    var id: Int32
    var title: String
    var detail: String

    init(id: Int32, title: String = "WireGuard", detail: String = "Stored configuration") {
        self.id = id
        self.title = title
        self.detail = detail
    }

    init(network: Burrow_Network) {
        let payload = String(data: network.payload, encoding: .utf8) ?? ""
        let address = Self.firstValue(for: "Address", in: payload)
        let endpoint = Self.firstValue(for: "Endpoint", in: payload)
        self.id = network.id
        self.title = "WireGuard"
        self.detail = [address, endpoint]
            .compactMap { $0 }
            .filter { !$0.isEmpty }
            .joined(separator: " · ")
            .ifEmpty("Stored configuration")
    }

    var card: NetworkCardModel {
        NetworkCardModel(
            id: id,
            backgroundColor: .init("WireGuard"),
            label: AnyView(label)
        )
    }

    private var label: some View {
        GeometryReader { reader in
            VStack(alignment: .leading) {
                HStack {
                    Image("WireGuard")
                        .resizable()
                        .aspectRatio(contentMode: .fit)
                    Image("WireGuardTitle")
                        .resizable()
                        .aspectRatio(contentMode: .fit)
                        .frame(width: reader.size.width / 2)
                    Spacer()
                }
                .frame(maxWidth: .infinity, maxHeight: reader.size.height / 4)
                Spacer()
                Text(detail)
                    .foregroundStyle(.white)
                    .font(.body.monospaced())
                    .lineLimit(3)
            }
            .padding()
            .frame(maxWidth: .infinity)
        }
    }

    private static func firstValue(for key: String, in config: String) -> String? {
        config
            .split(whereSeparator: \.isNewline)
            .map(String.init)
            .first(where: { $0.hasPrefix("\(key) = ") })?
            .split(separator: "=", maxSplits: 1)
            .last
            .map { $0.trimmingCharacters(in: .whitespaces) }
    }
}

private extension String {
    func ifEmpty(_ fallback: @autoclosure () -> String) -> String {
        isEmpty ? fallback() : self
    }
}
