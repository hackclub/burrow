import BurrowClient
import SwiftUI

struct WireGuard: Network {
    static let type: BurrowClient.Burrow_NetworkType = .wireGuard
    typealias NetworkType = Burrow_WireGuardNetwork

    var id: Int32
    var backgroundColor: Color { .init("WireGuard") }

    @MainActor
    var label: some View {
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
                Text("@conradev")
                    .foregroundStyle(.white)
                    .font(.body.monospaced())
            }
            .padding()
            .frame(maxWidth: .infinity)
        }
    }
}
