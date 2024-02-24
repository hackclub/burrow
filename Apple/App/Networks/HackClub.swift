import SwiftUI

struct HackClub: Network {
    var id: String
    var backgroundColor: Color { .init("HackClub") }

    var label: some View {
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
