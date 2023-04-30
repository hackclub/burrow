import SwiftUI

struct ContentView: View {

    @ObservedObject var viewModel = NetworkConfiguration()

    var body: some View {
        VStack(alignment: .leading) {
            
            HStack {
                Text(verbatim: "Networks \(viewModel.model.status)")
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
                    if (viewModel.status == .unknown) {
                        
                        
                        Image("hackClubLogo")
                            .resizable()
                            .frame(width: 32, height: 32)
                            .cornerRadius(100).onTapGesture {
                                viewModel.connectToBurrow()
                                print(viewModel.status)
                            }
                        
                    } else if (viewModel.status == .loading) {
                        ZStack {
                            Image("hackClubLogo")
                                .resizable()
                                .frame(width: 32, height: 32)
                                .cornerRadius(100)
                                .overlay(Color.white.opacity(0.6).cornerRadius(100))
                            
                            ProgressView()
                                .progressViewStyle(CircularProgressViewStyle())
                                .scaleEffect(0.6)
                        }
                    }
                    VStack(alignment: .leading) {
                        
                        Text("Hack Club Network")
                            .fontWeight(.medium)


                        Text("􁠲 Recently Validated Certificate")
                            .font(.caption2)
                            .foregroundColor(.blue)
                    }.onTapGesture {
                        print(true)
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
