import SwiftUI
import NetworkExtension

@MainActor
class NetworkConfiguration: ObservableObject {
    enum Status: CustomStringConvertible {
        case unknown
        case blank
        case valid
        case error
        
        var description: String {
            switch self {
            case .unknown:
                return "Unknown"
            case .blank:
                return "Blank"
            case .valid:
                return "Valid"
            default:
                return "Default"
            }
        }
    }
    
    @Published
    var status: Status = .unknown
    
    init() {
        update()
        

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
