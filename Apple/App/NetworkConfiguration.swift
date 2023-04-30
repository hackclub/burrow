import SwiftUI
import NetworkExtension

@MainActor
class NetworkConfiguration: ObservableObject {
    func connectToBurrow() {
        objectWillChange.send()
        model.connectToBurrow()
    }
    
    @Published var model = Model()
    
    @Published
    var status: Status = .unknown
    
    init() {
        update()
        

    }
    
    func connectToNetwork() {
        print(self.status)
        self.status = .loading
        print(self.status)

        DispatchQueue.main.asyncAfter(deadline: .now() + 3) {
            let random = Int.random(in: 0...1)
            if random == 0 {
                self.status = .valid
                print(self.status)

            } else {
                self.status = .error
                print(self.status)

            }
        }
    }
    
    func update() {
        Task {
            do {
                let configurations = try await NETunnelProviderManager.loadAll()
                
                await MainActor.run {
                    self.status = configurations.isEmpty ? .blank : .valid
                    print(self.status)
                    self.objectWillChange.send()
                }
            } catch {
                await MainActor.run {
                    self.status = .error
                    self.objectWillChange.send()

                }
            }
        }
    }
    
    func request() {
        let configuration = NETunnelProviderProtocol()
        configuration.providerBundleIdentifier = ""
        configuration.serverAddress = "Hack Club"

        let manager = NETunnelProviderManager()
        manager.protocolConfiguration = configuration
        manager.localizedDescription = "Hack Club Burrow"
        manager.saveToPreferences { error in
            print(error)
        }
    }
}

extension NETunnelProviderManager {
    static func loadAll() async throws -> [NETunnelProviderManager] {
        try await withUnsafeThrowingContinuation { continuation in
            NETunnelProviderManager.loadAllFromPreferences { managers, error in
                if let error = error {
                    continuation.resume(throwing: error)
                } else {
                    continuation.resume(returning: managers ?? [])
                }
            }
        }
    }
}
