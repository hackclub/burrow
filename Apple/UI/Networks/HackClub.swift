import BurrowCore
import SwiftUI

struct HackClub: Network {
    typealias NetworkType = Burrow_WireGuardNetwork
    static let type: Burrow_NetworkType = .hackClub

    var id: Int32
    var backgroundColor: Color { .init("HackClub") }

    @MainActor var label: some View {
        GeometryReader { reader in
            VStack(alignment: .leading) {
                Image("HackClub")
                    .resizable()
                    .aspectRatio(contentMode: .fit)
                    .frame(height: reader.size.height / 4)
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
