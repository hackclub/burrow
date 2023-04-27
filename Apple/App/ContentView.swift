import SwiftUI

struct ContentView: View {
    var body: some View {
        VStack(alignment: .leading) {
            
            HStack {
                
                
                Text("Networks")
                    .font(.title3)
                Spacer()
                Image(systemName: "badge.plus.radiowaves.forward")
                    .symbolRenderingMode(.palette)
                    .foregroundStyle(.blue, .black)
                .imageScale(.large)                }
            Divider()
            VStack(alignment: .leading) {
                Text("Your Burrows")
                    .padding(.top, 2)
                    .font(.subheadline)
                    .fontWeight(.bold)
                HStack {
                    Image("sampleNetworkIcon")
                        .resizable()
                        .frame(width: 32, height: 32)
                        .cornerRadius(100)
                    VStack(alignment: .leading) {
                        
                        Text("Penguin Pair Burrow")
                            .fontWeight(.medium)


                        Text("􁠲 Recently Validated Certificate")
                            .font(.caption2)
                            .foregroundColor(.blue)
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
