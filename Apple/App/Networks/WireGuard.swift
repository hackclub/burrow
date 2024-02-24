import SwiftUI

struct WireGuard: Network {
    var id: String
    var backgroundColor: Color { .init("WireGuard") }

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
