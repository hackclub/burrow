import SwiftUI

struct ContentView: View {
    @State private var connectedHackClubNetwork = false
    
    var body: some View {
        VStack(alignment: .leading) {
            
            HStack {
                Text("Networks")
                    .font(.title3)
                Spacer()
                Image(systemName: "badge.plus.radiowaves.forward")
                    .symbolRenderingMode(.palette)
                    .foregroundStyle(.blue, .black)
                    .opacity(0.4)
                .imageScale(.large)                }
            Divider()
            VStack(alignment: .leading) {
                Text("Burrows")
                    .padding(.top, 2)
                    .font(.subheadline.weight(.bold))
                HStack {
                    Image("hackClubLogo")
                        .resizable()
                        .frame(width: 32, height: 32)
                        .cornerRadius(100)
                    VStack(alignment: .leading) {
                        
                        Text("Hack Club Network")
                            .fontWeight(.medium)


                        Text("􁠲 Recently Validated Certificate")
                            .font(.caption2)
                            .foregroundColor(.blue)
                    }.onTapGesture {
                        connectedHackClubNetwork = true
                    }
                }
            }

        }
            .padding()
    }
}

struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView()
    }
}
